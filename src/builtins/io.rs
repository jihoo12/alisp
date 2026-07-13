use std::io::{self, Write};
use crate::Evaluator;
use crate::expr::{Expr, expr_to_string};

impl Evaluator {
    pub(crate) fn builtin_print(&mut self, args: &[Expr], newline: bool) -> Result<Expr, String> {
        let s: String = args.iter().map(expr_to_string).collect::<Vec<_>>().join(" ");
        if newline {
            println!("{}", s);
        } else {
            print!("{}", s);
            io::stdout().flush().ok();
        }
        Ok(Expr::Nil)
    }

    pub(crate) fn builtin_eprint(&mut self, args: &[Expr], newline: bool) -> Result<Expr, String> {
        let s: String = args.iter().map(expr_to_string).collect::<Vec<_>>().join(" ");
        if newline {
            eprintln!("{}", s);
        } else {
            eprint!("{}", s);
            io::stderr().flush().ok();
        }
        Ok(Expr::Nil)
    }

    pub(crate) fn builtin_input(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if !args.is_empty() {
            let prompt = expr_to_string(&args[0]);
            eprint!("{}", prompt);
            io::stderr().flush().ok();
        }
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| format!("input error: {}", e))?;
        Ok(Expr::Str(line.trim_end_matches('\n').to_string()))
    }
}
