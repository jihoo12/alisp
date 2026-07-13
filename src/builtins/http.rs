use std::process::Command;
use crate::Evaluator;
use crate::expr::{Expr, expr_to_string};

impl Evaluator {
    pub(crate) fn builtin_http(&mut self, args: &[Expr], method: &str) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("http-get requires a URL".into());
        }
        let url = expr_to_string(&args[0]);
        let output = Command::new("curl")
            .args(["-s", "-X", method, &url])
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("HTTP {} failed: {}", method, stderr.trim()));
        }
        Ok(Expr::Str(String::from_utf8_lossy(&output.stdout).to_string()))
    }

    pub(crate) fn builtin_http_with_body(&mut self, args: &[Expr], method: &str) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err(format!("(http-{} url body)", method.to_lowercase()));
        }
        let url = expr_to_string(&args[0]);
        let body = expr_to_string(&args[1]);
        let output = Command::new("curl")
            .args(["-s", "-X", method, &url, "-d", &body])
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("HTTP {} failed: {}", method, stderr.trim()));
        }
        Ok(Expr::Str(String::from_utf8_lossy(&output.stdout).to_string()))
    }

    pub(crate) fn builtin_http_request(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(http method url ?body? ?headers?)".into());
        }
        let method = expr_to_string(&args[0]);
        let url = expr_to_string(&args[1]);
        let mut cmd_args = vec!["-s".to_string(), "-X".to_string(), method.clone(), url.clone()];
        if args.len() >= 3 {
            let body = expr_to_string(&args[2]);
            cmd_args.push("-d".to_string());
            cmd_args.push(body);
        }
        if args.len() >= 4 {
            if let Expr::List(headers) = &args[3] {
                for h in headers {
                    cmd_args.push("-H".to_string());
                    cmd_args.push(expr_to_string(h));
                }
            }
        }
        let output = Command::new("curl")
            .args(&cmd_args)
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("HTTP {} failed: {}", method, stderr.trim()));
        }
        Ok(Expr::Str(String::from_utf8_lossy(&output.stdout).to_string()))
    }
}
