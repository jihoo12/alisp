use crate::Evaluator;
use crate::expr::Expr;

impl Evaluator {
    pub(crate) fn builtin_sleep(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() { return Err("sleep requires duration in seconds".into()); }
        let secs = match &args[0] {
            Expr::Num(n) => *n,
            _ => return Err("sleep: argument must be a number".into()),
        };
        std::thread::sleep(std::time::Duration::from_secs_f64(secs));
        Ok(Expr::Nil)
    }

    pub(crate) fn builtin_time(&mut self, _args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Num(self.start_time.elapsed().as_secs_f64()))
    }

    pub(crate) fn builtin_timestamp(&mut self, _args: &[Expr]) -> Result<Expr, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("timestamp: {}", e))?;
        Ok(Expr::Num(now.as_secs_f64()))
    }

    pub(crate) fn builtin_exit(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let code = if args.is_empty() { 0 } else {
            match &args[0] {
                Expr::Num(n) => *n as i32,
                _ => 0,
            }
        };
        std::process::exit(code);
    }
}
