use std::collections::HashMap;
use std::fs;

use crate::token::tokenize;
use crate::parser::Parser;
use crate::expr::{Expr, is_truthy, expr_to_string};

pub struct Evaluator {
    pub(crate) globals: HashMap<String, Expr>,
    pub(crate) locals: Vec<HashMap<String, Expr>>,
    pub(crate) start_time: std::time::Instant,
}

impl Evaluator {
    pub fn new() -> Self {
        let mut globals = HashMap::new();
        let builtins: &[&str] = &[
            "exec", "shell", "sh", "exec-result",
            "read", "read-file", "read-lines", "write", "write-file", "append", "append-file",
            "exists", "file-exists", "file?", "dir?", "file-size", "mtime",
            "touch", "rm", "delete", "mkdir", "cp", "copy", "mv", "move",
            "ls", "list-dir", "glob", "cwd", "pwd", "cd",
            "basename", "dirname", "ext", "join-path", "realpath",
            "getenv", "env-get", "setenv", "env-set", "env",
            "str", "split", "join", "trim", "contains", "includes", "starts-with", "ends-with",
            "replace", "upper", "lower", "substr", "find", "format",
            "list", "car", "head", "first", "cdr", "tail", "rest", "cons",
            "len", "length", "size", "push", "nth", "at",
            "map", "filter", "select", "reduce", "fold", "each", "for-each",
            "range", "reverse", "sort", "flatten", "last", "empty?",
            "any", "all", "zip", "assoc", "dissoc", "keys", "values", "merge",
            "+", "-", "*", "/", "%", "mod", "pow", "sqrt", "abs", "min", "max",
            "floor", "ceil", "round", "rand", "random", "inc", "dec",
            "=", "==", "!=", "<", ">", "<=", ">=", "not",
            "type", "type-of", "int", "float",
            "number?", "string?", "list?", "nil?", "bool?",
            "print", "println", "eprint", "eprintln", "input",
            "http-get", "http-post", "http-put", "http-delete", "http",
            "json-parse", "json", "json-stringify", "json-str", "json-get", "jget",
            "json-set", "jset", "json-keys",
            "sleep", "time", "timestamp", "exit", "quit",
            "re-test", "re-match?", "re-match",
            "re-find", "re-find-all",
            "re-replace", "re-replace-all",
            "re-split", "re-scan",
        ];
        for &name in builtins {
            globals.insert(name.to_string(), Expr::Builtin(name));
        }
        Evaluator {
            globals,
            locals: Vec::new(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn get(&self, name: &str) -> Option<Expr> {
        for scope in self.locals.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v.clone());
            }
        }
        self.globals.get(name).cloned()
    }

    pub fn set_global(&mut self, name: String, val: Expr) {
        self.globals.insert(name, val);
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Expr, String> {
        match expr {
            Expr::Num(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Nil | Expr::Lambda { .. } | Expr::Builtin(_) => {
                Ok(expr.clone())
            }
            Expr::Sym(name) => self.get(name).ok_or_else(|| format!("Undefined: {}", name)),
            Expr::List(list) => {
                if list.is_empty() {
                    return Ok(Expr::Nil);
                }
                self.eval_list(list)
            }
        }
    }

    pub fn eval_str(&mut self, code: &str) -> Result<Option<Expr>, String> {
        let tokens = tokenize(code)?;
        if tokens.is_empty() {
            return Ok(None);
        }
        let mut parser = Parser::new(tokens);
        let exprs = parser.parse_all()?;
        if exprs.is_empty() {
            return Ok(None);
        }
        let mut result = Expr::Nil;
        for expr in &exprs {
            result = self.eval(expr)?;
        }
        Ok(Some(result))
    }

    pub fn eval_file(&mut self, path: &str) -> Result<Option<Expr>, String> {
        let code = fs::read_to_string(path).map_err(|e| format!("read '{}': {}", path, e))?;
        self.eval_str(&code)
    }

    pub(crate) fn eval_list(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if let Expr::Sym(s) = &list[0] {
            match s.as_str() {
                "def" => return self.eval_def(list),
                "set!" => return self.eval_set(list),
                "fn" | "lambda" => return self.eval_lambda(list),
                "defn" => return self.eval_defn(list),
                "if" => return self.eval_if(list),
                "when" => return self.eval_when(list),
                "unless" => return self.eval_unless(list),
                "cond" => return self.eval_cond(list),
                "do" | "begin" => return self.eval_do(list),
                "let" => return self.eval_let(list),
                "while" => return self.eval_while(list),
                "dolist" => return self.eval_dolist(list),
                "dotimes" => return self.eval_dotimes(list),
                "quote" => return Ok(if list.len() > 1 {
                    list[1].clone()
                } else {
                    Expr::Nil
                }),
                "try" => return self.eval_try(list),
                "throw" => return self.eval_throw(list),
                "apply" => return self.eval_apply(list),
                "eval" => return self.eval_eval(list),
                "and" => {
                    let mut result = Expr::Bool(true);
                    for arg in &list[1..] {
                        result = self.eval(arg)?;
                        if !is_truthy(&result) {
                            return Ok(result);
                        }
                    }
                    return Ok(result);
                }
                "or" => {
                    let mut result = Expr::Bool(false);
                    for arg in &list[1..] {
                        result = self.eval(arg)?;
                        if is_truthy(&result) {
                            return Ok(result);
                        }
                    }
                    return Ok(result);
                }
                _ => {}
            }
        }

        let func = self.eval(&list[0])?;
        let mut args = Vec::with_capacity(list.len() - 1);
        for arg in &list[1..] {
            args.push(self.eval(arg)?);
        }
        self.call(&func, &args)
    }

    pub(crate) fn call(&mut self, func: &Expr, args: &[Expr]) -> Result<Expr, String> {
        match func {
            Expr::Lambda { params, body } => {
                if args.len() != params.len() {
                    return Err(format!(
                        "Expected {} args, got {}",
                        params.len(),
                        args.len()
                    ));
                }
                let mut scope = HashMap::new();
                for (p, a) in params.iter().zip(args) {
                    scope.insert(p.clone(), a.clone());
                }
                self.locals.push(scope);
                let mut result = Expr::Nil;
                for expr in body {
                    result = self.eval(expr)?;
                }
                self.locals.pop();
                Ok(result)
            }
            Expr::Builtin(name) => self.call_builtin(name, args),
            _ => Err(format!("Not callable: {}", expr_to_string(func))),
        }
    }

    // ===================== SPECIAL FORMS =====================

    fn eval_def(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() != 3 {
            return Err("(def name value)".into());
        }
        let name = match &list[1] {
            Expr::Sym(s) => s.clone(),
            _ => return Err("def: first arg must be symbol".into()),
        };
        let val = self.eval(&list[2])?;
        self.set_global(name, val.clone());
        Ok(val)
    }

    fn eval_set(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() != 3 {
            return Err("(set! name value)".into());
        }
        let name = match &list[1] {
            Expr::Sym(s) => s.clone(),
            _ => return Err("set!: first arg must be symbol".into()),
        };
        let val = self.eval(&list[2])?;
        let mut found = false;
        for scope in self.locals.iter_mut().rev() {
            if scope.contains_key(&name) {
                scope.insert(name.clone(), val.clone());
                found = true;
                break;
            }
        }
        if !found {
            self.globals.insert(name, val.clone());
        }
        Ok(val)
    }

    fn eval_lambda(&self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 3 {
            return Err("(fn (params) body...)".into());
        }
        let params: Vec<String> = match &list[1] {
            Expr::List(l) => l
                .iter()
                .map(|p| -> Result<String, String> {
                    match p {
                        Expr::Sym(s) => Ok(s.clone()),
                        _ => Err("Lambda param must be symbol".into()),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err("Lambda params must be a list".into()),
        };
        let body = list[2..].to_vec();
        Ok(Expr::Lambda { params, body })
    }

    fn eval_defn(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 4 {
            return Err("(defn name (params) body...)".into());
        }
        let name = match &list[1] {
            Expr::Sym(s) => s.clone(),
            _ => return Err("defn: name must be symbol".into()),
        };
        let mut lambda_parts = vec![Expr::Sym("fn".into()), list[2].clone()];
        lambda_parts.extend_from_slice(&list[3..]);
        let lambda = self.eval_lambda(&lambda_parts)?;
        self.set_global(name, lambda);
        Ok(Expr::Nil)
    }

    fn eval_if(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 3 {
            return Err("(if cond then else?)".into());
        }
        if is_truthy(&self.eval(&list[1])?) {
            self.eval(&list[2])
        } else if list.len() > 3 {
            self.eval(&list[3])
        } else {
            Ok(Expr::Nil)
        }
    }

    fn eval_when(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 2 {
            return Err("(when cond body...)".into());
        }
        if is_truthy(&self.eval(&list[1])?) {
            self.eval_do(&{
                let mut v = vec![Expr::Sym("do".into())];
                v.extend_from_slice(&list[2..]);
                v
            })
        } else {
            Ok(Expr::Nil)
        }
    }

    fn eval_unless(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 2 {
            return Err("(unless cond body...)".into());
        }
        if !is_truthy(&self.eval(&list[1])?) {
            self.eval_do(&{
                let mut v = vec![Expr::Sym("do".into())];
                v.extend_from_slice(&list[2..]);
                v
            })
        } else {
            Ok(Expr::Nil)
        }
    }

    fn eval_cond(&mut self, list: &[Expr]) -> Result<Expr, String> {
        for clause in &list[1..] {
            if let Expr::List(pair) = clause {
                if pair.len() == 2 {
                    if is_truthy(&self.eval(&pair[0])?) {
                        return self.eval(&pair[1]);
                    }
                } else if pair.len() == 1 {
                    return self.eval(&pair[0]);
                }
            }
        }
        Ok(Expr::Nil)
    }

    fn eval_do(&mut self, list: &[Expr]) -> Result<Expr, String> {
        let mut result = Expr::Nil;
        for e in &list[1..] {
            result = self.eval(e)?;
        }
        Ok(result)
    }

    fn eval_let(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 3 {
            return Err("(let ((name val) ...) body...)".into());
        }
        let bindings = match &list[1] {
            Expr::List(l) => l.clone(),
            _ => return Err("let: bindings must be a list".into()),
        };
        let mut scope = HashMap::new();
        for binding in &bindings {
            if let Expr::List(pair) = binding {
                if pair.len() != 2 {
                    return Err("let: each binding must be (name value)".into());
                }
                let name = match &pair[0] {
                    Expr::Sym(s) => s.clone(),
                    _ => return Err("let: binding name must be symbol".into()),
                };
                let val = self.eval(&pair[1])?;
                scope.insert(name, val);
            } else {
                return Err("let: each binding must be a list".into());
            }
        }
        self.locals.push(scope);
        let mut result = Expr::Nil;
        for e in &list[2..] {
            result = self.eval(e)?;
        }
        self.locals.pop();
        Ok(result)
    }

    fn eval_while(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 2 {
            return Err("(while cond body...)".into());
        }
        let mut result = Expr::Nil;
        loop {
            if !is_truthy(&self.eval(&list[1])?) {
                break;
            }
            for e in &list[2..] {
                result = self.eval(e)?;
            }
        }
        Ok(result)
    }

    fn eval_dolist(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 3 {
            return Err("(dolist (var list-expr) body...)".into());
        }
        let (var_name, list_expr) = match &list[1] {
            Expr::List(pair) if pair.len() == 2 => match (&pair[0], &pair[1]) {
                (Expr::Sym(s), e) => (s.clone(), e.clone()),
                _ => return Err("dolist: (var list-expr)".into()),
            },
            _ => return Err("dolist: (var list-expr)".into()),
        };
        let items = match self.eval(&list_expr)? {
            Expr::List(v) => v,
            _ => return Err("dolist: second arg must be a list".into()),
        };
        let mut result = Expr::Nil;
        for item in &items {
            let mut scope = HashMap::new();
            scope.insert(var_name.clone(), item.clone());
            self.locals.push(scope);
            for e in &list[2..] {
                result = self.eval(e)?;
            }
            self.locals.pop();
        }
        Ok(result)
    }

    fn eval_dotimes(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 3 {
            return Err("(dotimes (var n) body...)".into());
        }
        let (var_name, n_expr) = match &list[1] {
            Expr::List(pair) if pair.len() == 2 => match (&pair[0], &pair[1]) {
                (Expr::Sym(s), e) => (s.clone(), e.clone()),
                _ => return Err("dotimes: (var n-expr)".into()),
            },
            _ => return Err("dotimes: (var n-expr)".into()),
        };
        let n = match self.eval(&n_expr)? {
            Expr::Num(n) => n as i64,
            _ => return Err("dotimes: n must be a number".into()),
        };
        let mut result = Expr::Nil;
        for i in 0..n {
            let mut scope = HashMap::new();
            scope.insert(var_name.clone(), Expr::Num(i as f64));
            self.locals.push(scope);
            for e in &list[2..] {
                result = self.eval(e)?;
            }
            self.locals.pop();
        }
        Ok(result)
    }

    fn eval_try(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 2 {
            return Err("(try body... (catch e handler...))".into());
        }
        let mut try_body = Vec::new();
        let mut catch_var = String::from("_");
        let mut catch_body = Vec::new();

        for e in &list[1..] {
            if let Expr::List(l) = e {
                if let Expr::Sym(s) = &l[0] {
                    if s == "catch" {
                        if l.len() >= 2 {
                            catch_var = match &l[1] {
                                Expr::Sym(s) => s.clone(),
                                _ => "_".into(),
                            };
                            catch_body = l[2..].to_vec();
                        }
                        continue;
                    }
                }
            }
            try_body.push(e.clone());
        }

        let mut last = Expr::Nil;
        for e in &try_body {
            match self.eval(e) {
                Ok(v) => last = v,
                Err(err) => {
                    if !catch_body.is_empty() {
                        let mut scope = HashMap::new();
                        scope.insert(catch_var, Expr::Str(err));
                        self.locals.push(scope);
                        let mut result = Expr::Nil;
                        for ce in &catch_body {
                            result = self.eval(ce)?;
                        }
                        self.locals.pop();
                        return Ok(result);
                    }
                    return Err(err);
                }
            }
        }
        Ok(last)
    }

    fn eval_throw(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() < 2 {
            return Err("(throw message)".into());
        }
        let msg = expr_to_string(&self.eval(&list[1])?);
        Err(msg)
    }

    fn eval_apply(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() != 3 {
            return Err("(apply func args)".into());
        }
        let func = self.eval(&list[1])?;
        let args = match self.eval(&list[2])? {
            Expr::List(v) => v,
            _ => return Err("apply: second arg must be a list".into()),
        };
        self.call(&func, &args)
    }

    fn eval_eval(&mut self, list: &[Expr]) -> Result<Expr, String> {
        if list.len() != 2 {
            return Err("(eval expr)".into());
        }
        let code = expr_to_string(&self.eval(&list[1])?);
        let tokens = tokenize(&code)?;
        let mut parser = Parser::new(tokens);
        let exprs = parser.parse_all()?;
        let mut result = Expr::Nil;
        for e in exprs {
            result = self.eval(&e)?;
        }
        Ok(result)
    }
}
