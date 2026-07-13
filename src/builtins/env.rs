use std::env;
use crate::Evaluator;
use crate::expr::{Expr, expr_to_string};

impl Evaluator {
    pub(crate) fn builtin_getenv(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("getenv requires a name".into());
        }
        let name = expr_to_string(&args[0]);
        Ok(match env::var(&name) {
            Ok(v) => Expr::Str(v),
            Err(_) => Expr::Nil,
        })
    }

    pub(crate) fn builtin_setenv(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(setenv name value)".into());
        }
        let name = expr_to_string(&args[0]);
        let value = expr_to_string(&args[1]);
        unsafe { env::set_var(&name, &value) };
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_env(&mut self, _args: &[Expr]) -> Result<Expr, String> {
        let pairs: Vec<Expr> = env::vars()
            .map(|(k, v)| Expr::List(vec![Expr::Str(k), Expr::Str(v)]))
            .collect();
        Ok(Expr::List(pairs))
    }
}
