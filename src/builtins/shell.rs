use std::process::Command;
use crate::Evaluator;
use crate::expr::{Expr, expr_to_string};

impl Evaluator {
    pub(crate) fn builtin_exec(&mut self, args: &[Expr], return_result: bool) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("exec requires a command".into());
        }
        let cmd: String = args.iter().map(expr_to_string).collect::<Vec<_>>().join(" ");
        let output = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .output()
            .map_err(|e| format!("exec failed: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if return_result {
            let status = output.status.code().unwrap_or(-1);
            Ok(Expr::List(vec![
                Expr::List(vec![Expr::Str("status".into()), Expr::Num(status as f64)]),
                Expr::List(vec![Expr::Str("stdout".into()), Expr::Str(stdout.trim().into())]),
                Expr::List(vec![Expr::Str("stderr".into()), Expr::Str(stderr.trim().into())]),
            ]))
        } else {
            if !output.status.success() {
                let code = output.status.code().unwrap_or(-1);
                let err_msg = stderr.trim().to_string();
                if err_msg.is_empty() {
                    return Err(format!("Command '{}' failed (exit {})", cmd, code));
                }
                return Err(format!("{} (exit {})", err_msg, code));
            }
            Ok(Expr::Str(stdout.trim().to_string()))
        }
    }
}
