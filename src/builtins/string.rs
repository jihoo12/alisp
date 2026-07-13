use crate::Evaluator;
use crate::expr::{Expr, expr_to_string};

impl Evaluator {
    pub(crate) fn builtin_str(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let result: String = args.iter().map(expr_to_string).collect();
        Ok(Expr::Str(result))
    }

    pub(crate) fn builtin_split(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(split string delimiter)".into());
        }
        let s = expr_to_string(&args[0]);
        let delim = expr_to_string(&args[1]);
        let parts: Vec<Expr> = s.split(&*delim).map(|s| Expr::Str(s.to_string())).collect();
        Ok(Expr::List(parts))
    }

    pub(crate) fn builtin_join(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(join list delimiter)".into());
        }
        let delim = expr_to_string(&args[1]);
        let items = match &args[0] {
            Expr::List(v) => v,
            _ => return Err("join: first arg must be a list".into()),
        };
        let result: String = items
            .iter()
            .map(expr_to_string)
            .collect::<Vec<_>>()
            .join(&delim);
        Ok(Expr::Str(result))
    }

    pub(crate) fn builtin_trim(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("trim requires a string".into());
        }
        Ok(Expr::Str(expr_to_string(&args[0]).trim().to_string()))
    }

    pub(crate) fn builtin_contains(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(contains haystack needle)".into());
        }
        let haystack = expr_to_string(&args[0]);
        let needle = expr_to_string(&args[1]);
        Ok(Expr::Bool(haystack.contains(&*needle)))
    }

    pub(crate) fn builtin_starts_with(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(starts-with s prefix)".into());
        }
        let s = expr_to_string(&args[0]);
        let prefix = expr_to_string(&args[1]);
        Ok(Expr::Bool(s.starts_with(&*prefix)))
    }

    pub(crate) fn builtin_ends_with(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(ends-with s suffix)".into());
        }
        let s = expr_to_string(&args[0]);
        let suffix = expr_to_string(&args[1]);
        Ok(Expr::Bool(s.ends_with(&*suffix)))
    }

    pub(crate) fn builtin_replace(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 {
            return Err("(replace s old new)".into());
        }
        let s = expr_to_string(&args[0]);
        let old = expr_to_string(&args[1]);
        let new = expr_to_string(&args[2]);
        Ok(Expr::Str(s.replace(&*old, &*new)))
    }

    pub(crate) fn builtin_upper(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("upper requires a string".into());
        }
        Ok(Expr::Str(expr_to_string(&args[0]).to_uppercase()))
    }

    pub(crate) fn builtin_lower(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("lower requires a string".into());
        }
        Ok(Expr::Str(expr_to_string(&args[0]).to_lowercase()))
    }

    pub(crate) fn builtin_substr(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 {
            return Err("(substr s start len)".into());
        }
        let s = expr_to_string(&args[0]);
        let start = match &args[1] {
            Expr::Num(n) => *n as usize,
            _ => return Err("substr: start must be a number".into()),
        };
        let len = match &args[2] {
            Expr::Num(n) => *n as usize,
            _ => return Err("substr: len must be a number".into()),
        };
        let end = (start + len).min(s.len());
        if start >= s.len() {
            return Ok(Expr::Str(String::new()));
        }
        Ok(Expr::Str(s[start..end].to_string()))
    }

    pub(crate) fn builtin_find(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(find haystack needle)".into());
        }
        let haystack = expr_to_string(&args[0]);
        let needle = expr_to_string(&args[1]);
        match haystack.find(&*needle) {
            Some(pos) => Ok(Expr::Num(pos as f64)),
            None => Ok(Expr::Num(-1.0)),
        }
    }

    pub(crate) fn builtin_format(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("format requires a template".into());
        }
        let template = expr_to_string(&args[0]);
        let mut result = template;
        for (i, arg) in args[1..].iter().enumerate() {
            let numbered = format!("{{{}}}", i);
            result = result.replace(&numbered, &expr_to_string(arg));
        }
        let mut arg_idx = 0;
        while let Some(pos) = result.find("{}") {
            if arg_idx < args.len() - 1 {
                let replacement = expr_to_string(&args[1 + arg_idx]);
                result = format!("{}{}{}", &result[..pos], replacement, &result[pos + 2..]);
                arg_idx += 1;
            } else {
                break;
            }
        }
        Ok(Expr::Str(result))
    }
}
