use crate::Evaluator;
use crate::expr::{Expr, expr_to_string, is_truthy};
use crate::json::json_is_object;

impl Evaluator {
    pub(crate) fn builtin_list_fn(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::List(args.to_vec()))
    }

    pub(crate) fn builtin_car(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("car requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => Ok(v.first().cloned().unwrap_or(Expr::Nil)),
            Expr::Str(s) => Ok(s
                .chars()
                .next()
                .map(|c| Expr::Str(c.to_string()))
                .unwrap_or(Expr::Nil)),
            _ => Err("car: argument must be a list or string".into()),
        }
    }

    pub(crate) fn builtin_cdr(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("cdr requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => Ok(Expr::List(v[1..].to_vec())),
            Expr::Str(s) => Ok(Expr::Str(s.chars().skip(1).collect())),
            _ => Err("cdr: argument must be a list or string".into()),
        }
    }

    pub(crate) fn builtin_cons(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(cons elem list)".into());
        }
        match &args[1] {
            Expr::List(v) => {
                let mut result = vec![args[0].clone()];
                result.extend_from_slice(v);
                Ok(Expr::List(result))
            }
            Expr::Nil => Ok(Expr::List(vec![args[0].clone()])),
            _ => Err("cons: second arg must be a list".into()),
        }
    }

    pub(crate) fn builtin_len(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("len requires an argument".into());
        }
        let n = match &args[0] {
            Expr::List(v) => v.len(),
            Expr::Str(s) => s.len(),
            _ => return Err("len: argument must be a list or string".into()),
        };
        Ok(Expr::Num(n as f64))
    }

    pub(crate) fn builtin_push(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(push list elem)".into());
        }
        match &args[0] {
            Expr::List(v) => {
                let mut result = v.clone();
                result.push(args[1].clone());
                Ok(Expr::List(result))
            }
            _ => Err("push: first arg must be a list".into()),
        }
    }

    pub(crate) fn builtin_nth(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(nth list index)".into());
        }
        let idx = match &args[1] {
            Expr::Num(n) => *n as usize,
            _ => return Err("nth: index must be a number".into()),
        };
        match &args[0] {
            Expr::List(v) => Ok(v.get(idx).cloned().unwrap_or(Expr::Nil)),
            Expr::Str(s) => Ok(s
                .chars()
                .nth(idx)
                .map(|c| Expr::Str(c.to_string()))
                .unwrap_or(Expr::Nil)),
            _ => Err("nth: first arg must be a list or string".into()),
        }
    }

    pub(crate) fn builtin_map(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(map func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v,
            _ => return Err("map: second arg must be a list".into()),
        };
        let mut result = Vec::with_capacity(items.len());
        for item in items {
            let mapped = self.call(func, &[item.clone()])?;
            result.push(mapped);
        }
        Ok(Expr::List(result))
    }

    pub(crate) fn builtin_filter(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(filter func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v,
            _ => return Err("filter: second arg must be a list".into()),
        };
        let mut result = Vec::new();
        for item in items {
            if is_truthy(&self.call(func, &[item.clone()])?) {
                result.push(item.clone());
            }
        }
        Ok(Expr::List(result))
    }

    pub(crate) fn builtin_reduce(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 {
            return Err("(reduce func init list)".into());
        }
        let func = &args[0];
        let mut acc = args[1].clone();
        let items = match &args[2] {
            Expr::List(v) => v,
            _ => return Err("reduce: third arg must be a list".into()),
        };
        for item in items {
            acc = self.call(func, &[acc, item.clone()])?;
        }
        Ok(acc)
    }

    pub(crate) fn builtin_each(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(each func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v.clone(),
            _ => return Err("each: second arg must be a list".into()),
        };
        for item in &items {
            self.call(func, &[item.clone()])?;
        }
        Ok(Expr::Nil)
    }

    pub(crate) fn builtin_range(&mut self, args: &[Expr]) -> Result<Expr, String> {
        match args.len() {
            1 => {
                let end = match &args[0] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: argument must be a number".into()),
                };
                let start: i64 = if end > 0 { 0 } else { end };
                let step: i64 = if end > 0 { 1 } else { -1 };
                let items: Vec<Expr> = (start..end)
                    .step_by(step.unsigned_abs() as usize)
                    .map(|i| Expr::Num(i as f64))
                    .collect();
                Ok(Expr::List(items))
            }
            2 => {
                let start = match &args[0] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                let end = match &args[1] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                let step: i64 = if end > start { 1 } else { -1 };
                let items: Vec<Expr> = (start..end)
                    .step_by(step.unsigned_abs() as usize)
                    .map(|i| Expr::Num(i as f64))
                    .collect();
                Ok(Expr::List(items))
            }
            3 => {
                let start = match &args[0] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                let end = match &args[1] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                let step = match &args[2] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                if step == 0 {
                    return Err("range: step cannot be zero".into());
                }
                let items: Vec<Expr> = if step > 0 {
                    (start..end).step_by(step as usize).map(|i| Expr::Num(i as f64)).collect()
                } else {
                    (end..=start).rev().step_by((-step) as usize).map(|i| Expr::Num(i as f64)).collect()
                };
                Ok(Expr::List(items))
            }
            _ => Err("(range end) or (range start end) or (range start end step)".into()),
        }
    }

    pub(crate) fn builtin_reverse(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("reverse requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => {
                let mut result = v.clone();
                result.reverse();
                Ok(Expr::List(result))
            }
            Expr::Str(s) => Ok(Expr::Str(s.chars().rev().collect())),
            _ => Err("reverse: argument must be a list or string".into()),
        }
    }

    pub(crate) fn builtin_sort(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("sort requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => {
                let mut result = v.clone();
                result.sort_by(|a, b| {
                    expr_to_string(a).cmp(&expr_to_string(b))
                });
                Ok(Expr::List(result))
            }
            _ => Err("sort: argument must be a list".into()),
        }
    }

    pub(crate) fn builtin_flatten(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("flatten requires a list".into());
        }
        fn flatten_list(e: &Expr) -> Vec<Expr> {
            match e {
                Expr::List(v) => v.iter().flat_map(flatten_list).collect(),
                other => vec![other.clone()],
            }
        }
        Ok(Expr::List(flatten_list(&args[0])))
    }

    pub(crate) fn builtin_last(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("last requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => Ok(v.last().cloned().unwrap_or(Expr::Nil)),
            Expr::Str(s) => Ok(s
                .chars()
                .last()
                .map(|c| Expr::Str(c.to_string()))
                .unwrap_or(Expr::Nil)),
            _ => Err("last: argument must be a list or string".into()),
        }
    }

    pub(crate) fn builtin_empty(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("empty? requires an argument".into());
        }
        let empty = match &args[0] {
            Expr::List(v) => v.is_empty(),
            Expr::Str(s) => s.is_empty(),
            Expr::Nil => true,
            _ => false,
        };
        Ok(Expr::Bool(empty))
    }

    pub(crate) fn builtin_any(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(any func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v,
            _ => return Err("any: second arg must be a list".into()),
        };
        for item in items {
            if is_truthy(&self.call(func, &[item.clone()])?) {
                return Ok(Expr::Bool(true));
            }
        }
        Ok(Expr::Bool(false))
    }

    pub(crate) fn builtin_all(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(all func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v,
            _ => return Err("all: second arg must be a list".into()),
        };
        for item in items {
            if !is_truthy(&self.call(func, &[item.clone()])?) {
                return Ok(Expr::Bool(false));
            }
        }
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_zip(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("zip requires at least one list".into());
        }
        let lists: Result<Vec<Vec<Expr>>, String> = args
            .iter()
            .map(|a| match a {
                Expr::List(v) => Ok(v.clone()),
                _ => Err("zip: all arguments must be lists".into()),
            })
            .collect();
        let lists = lists?;
        let min_len = lists.iter().map(|l| l.len()).min().unwrap_or(0);
        let result: Vec<Expr> = (0..min_len)
            .map(|i| Expr::List(lists.iter().map(|l| l[i].clone()).collect()))
            .collect();
        Ok(Expr::List(result))
    }

    pub(crate) fn builtin_assoc(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 {
            return Err("(assoc alist key value)".into());
        }
        let key = expr_to_string(&args[1]);
        let val = args[2].clone();
        let mut pairs = match &args[0] {
            Expr::List(v) => v.clone(),
            Expr::Nil => Vec::new(),
            _ => return Err("assoc: first arg must be a list".into()),
        };
        pairs.retain(|p| {
            if let Expr::List(pair) = p {
                if let Expr::Str(k) = &pair[0] {
                    return k != &key;
                }
            }
            true
        });
        pairs.push(Expr::List(vec![Expr::Str(key), val]));
        Ok(Expr::List(pairs))
    }

    pub(crate) fn builtin_dissoc(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(dissoc alist key)".into());
        }
        let key = expr_to_string(&args[1]);
        let mut pairs = match &args[0] {
            Expr::List(v) => v.clone(),
            _ => return Err("dissoc: first arg must be a list".into()),
        };
        pairs.retain(|p| {
            if let Expr::List(pair) = p {
                if let Expr::Str(k) = &pair[0] {
                    return k != &key;
                }
            }
            true
        });
        Ok(Expr::List(pairs))
    }

    pub(crate) fn builtin_keys(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("keys requires an object".into());
        }
        match &args[0] {
            Expr::List(v) => {
                if json_is_object(&args[0]) {
                    let keys: Vec<Expr> = v
                        .iter()
                        .filter_map(|p| {
                            if let Expr::List(pair) = p {
                                Some(pair[0].clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    Ok(Expr::List(keys))
                } else {
                    Ok(Expr::List(Vec::new()))
                }
            }
            _ => Err("keys: argument must be an object (list of pairs)".into()),
        }
    }

    pub(crate) fn builtin_values(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("values requires an object".into());
        }
        match &args[0] {
            Expr::List(v) => {
                if json_is_object(&args[0]) {
                    let vals: Vec<Expr> = v
                        .iter()
                        .filter_map(|p| {
                            if let Expr::List(pair) = p {
                                Some(pair[1].clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    Ok(Expr::List(vals))
                } else {
                    Ok(args[0].clone())
                }
            }
            _ => Err("values: argument must be an object (list of pairs)".into()),
        }
    }

    pub(crate) fn builtin_merge(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let mut result = Vec::new();
        for arg in args {
            if let Expr::List(v) = arg {
                result.extend_from_slice(v);
            }
        }
        Ok(Expr::List(result))
    }
}
