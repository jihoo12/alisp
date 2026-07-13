use crate::Evaluator;
use crate::expr::{Expr, expr_to_string, expr_to_num, expr_eq, is_truthy};

pub(crate) fn rand_f64() -> f64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut hasher = s.build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
    );
    (hasher.finish() % 1000000) as f64 / 1000000.0
}

impl Evaluator {
    fn num_args(&self, args: &[Expr]) -> Result<Vec<f64>, String> {
        args.iter()
            .map(|a| expr_to_num(a))
            .collect()
    }

    pub(crate) fn builtin_add(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Ok(Expr::Num(0.0));
        }
        if let Expr::Str(_) = &args[0] {
            let result: String = args.iter().map(expr_to_string).collect();
            return Ok(Expr::Str(result));
        }
        let nums = self.num_args(args)?;
        Ok(Expr::Num(nums.iter().sum()))
    }

    pub(crate) fn builtin_sub(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("- requires at least one argument".into());
        }
        if nums.len() == 1 {
            return Ok(Expr::Num(-nums[0]));
        }
        Ok(Expr::Num(nums[1..].iter().fold(nums[0], |acc, n| acc - n)))
    }

    pub(crate) fn builtin_mul(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        Ok(Expr::Num(nums.iter().product()))
    }

    pub(crate) fn builtin_div(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.len() < 2 {
            return Err("/ requires at least two arguments".into());
        }
        let result = nums[1..].iter().try_fold(nums[0], |acc, n| {
            if *n == 0.0 {
                Err("Division by zero".to_string())
            } else {
                Ok(acc / n)
            }
        })?;
        Ok(Expr::Num(result))
    }

    pub(crate) fn builtin_mod(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.len() < 2 {
            return Err("% requires at least two arguments".into());
        }
        if nums[1] == 0.0 {
            return Err("Modulo by zero".into());
        }
        Ok(Expr::Num(nums[0] % nums[1]))
    }

    pub(crate) fn builtin_pow(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.len() < 2 {
            return Err("pow requires two arguments".into());
        }
        Ok(Expr::Num(nums[0].powf(nums[1])))
    }

    pub(crate) fn builtin_sqrt(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("sqrt requires an argument".into());
        }
        Ok(Expr::Num(nums[0].sqrt()))
    }

    pub(crate) fn builtin_abs(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("abs requires an argument".into());
        }
        Ok(Expr::Num(nums[0].abs()))
    }

    pub(crate) fn builtin_min(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("min requires arguments".into());
        }
        Ok(Expr::Num(nums.iter().cloned().fold(f64::INFINITY, f64::min)))
    }

    pub(crate) fn builtin_max(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("max requires arguments".into());
        }
        Ok(Expr::Num(nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)))
    }

    pub(crate) fn builtin_floor_fn(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("floor requires an argument".into());
        }
        Ok(Expr::Num(nums[0].floor()))
    }

    pub(crate) fn builtin_ceil_fn(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("ceil requires an argument".into());
        }
        Ok(Expr::Num(nums[0].ceil()))
    }

    pub(crate) fn builtin_round_fn(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("round requires an argument".into());
        }
        Ok(Expr::Num(nums[0].round()))
    }

    pub(crate) fn builtin_rand(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            Ok(Expr::Num(rand_f64()))
        } else {
            let nums = self.num_args(args)?;
            let max = nums[0];
            Ok(Expr::Num((rand_f64() * max).floor()))
        }
    }

    pub(crate) fn builtin_inc(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Ok(Expr::Num(1.0));
        }
        Ok(Expr::Num(nums[0] + 1.0))
    }

    pub(crate) fn builtin_dec(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Ok(Expr::Num(-1.0));
        }
        Ok(Expr::Num(nums[0] - 1.0))
    }

    // ---- Comparison ----

    pub(crate) fn builtin_eq(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("= requires two arguments".into());
        }
        Ok(Expr::Bool(expr_eq(&args[0], &args[1])))
    }

    pub(crate) fn builtin_ne(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("!= requires two arguments".into());
        }
        Ok(Expr::Bool(!expr_eq(&args[0], &args[1])))
    }

    pub(crate) fn builtin_lt(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 { return Err("< requires two arguments".into()); }
        let a = expr_to_num(&args[0])?;
        let b = expr_to_num(&args[1])?;
        Ok(Expr::Bool(a < b))
    }

    pub(crate) fn builtin_gt(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 { return Err("> requires two arguments".into()); }
        let a = expr_to_num(&args[0])?;
        let b = expr_to_num(&args[1])?;
        Ok(Expr::Bool(a > b))
    }

    pub(crate) fn builtin_lte(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 { return Err("<= requires two arguments".into()); }
        let a = expr_to_num(&args[0])?;
        let b = expr_to_num(&args[1])?;
        Ok(Expr::Bool(a <= b))
    }

    pub(crate) fn builtin_gte(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 { return Err(">= requires two arguments".into()); }
        let a = expr_to_num(&args[0])?;
        let b = expr_to_num(&args[1])?;
        Ok(Expr::Bool(a >= b))
    }

    // ---- Logic ----

    pub(crate) fn builtin_not(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("not requires an argument".into());
        }
        Ok(Expr::Bool(!is_truthy(&args[0])))
    }

    // ---- Type ----

    pub(crate) fn builtin_type(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("type requires an argument".into());
        }
        let type_name = match &args[0] {
            Expr::Num(_) => "number",
            Expr::Str(_) => "string",
            Expr::Bool(_) => "bool",
            Expr::Nil => "nil",
            Expr::Sym(_) => "symbol",
            Expr::List(_) => "list",
            Expr::Lambda { .. } => "function",
            Expr::Builtin(_) => "builtin",
        };
        Ok(Expr::Str(type_name.into()))
    }

    pub(crate) fn builtin_int(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() { return Err("int requires an argument".into()); }
        match &args[0] {
            Expr::Num(n) => Ok(Expr::Num(*n as i64 as f64)),
            Expr::Str(s) => s
                .parse::<f64>()
                .map(|n| Expr::Num(n as i64 as f64))
                .map_err(|_| format!("Cannot convert '{}' to int", s)),
            _ => Err(format!("Cannot convert to int: {}", expr_to_string(&args[0]))),
        }
    }

    pub(crate) fn builtin_float(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() { return Err("float requires an argument".into()); }
        match &args[0] {
            Expr::Num(n) => Ok(Expr::Num(*n)),
            Expr::Str(s) => s
                .parse::<f64>()
                .map(Expr::Num)
                .map_err(|_| format!("Cannot convert '{}' to float", s)),
            _ => Err(format!("Cannot convert to float: {}", expr_to_string(&args[0]))),
        }
    }

    pub(crate) fn builtin_is_number(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(false, |a| matches!(a, Expr::Num(_)))))
    }
    pub(crate) fn builtin_is_string(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(false, |a| matches!(a, Expr::Str(_)))))
    }
    pub(crate) fn builtin_is_list(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(false, |a| matches!(a, Expr::List(_)))))
    }
    pub(crate) fn builtin_is_nil(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(true, |a| matches!(a, Expr::Nil))))
    }
    pub(crate) fn builtin_is_bool(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(false, |a| matches!(a, Expr::Bool(_)))))
    }
}
