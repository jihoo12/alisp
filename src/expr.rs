#[derive(Debug, Clone)]
pub enum Expr {
    Num(f64),
    Str(String),
    Bool(bool),
    Nil,
    Sym(String),
    List(Vec<Expr>),
    Lambda {
        params: Vec<String>,
        body: Vec<Expr>,
    },
    Builtin(&'static str),
}

pub fn is_truthy(e: &Expr) -> bool {
    match e {
        Expr::Nil => false,
        Expr::Bool(false) => false,
        Expr::Num(n) if *n == 0.0 => false,
        Expr::Str(s) if s.is_empty() => false,
        Expr::List(v) if v.is_empty() => false,
        _ => true,
    }
}

pub fn expr_to_string(e: &Expr) -> String {
    match e {
        Expr::Num(n) => {
            if *n == (*n as i64) as f64 && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        Expr::Str(s) => s.clone(),
        Expr::Bool(true) => "true".into(),
        Expr::Bool(false) => "false".into(),
        Expr::Nil => "nil".into(),
        Expr::Sym(s) => s.clone(),
        Expr::List(v) => {
            let items: Vec<String> = v.iter().map(expr_to_string).collect();
            format!("({})", items.join(" "))
        }
        Expr::Lambda { params, .. } => format!("<fn ({})>", params.join(" ")),
        Expr::Builtin(name) => format!("<builtin:{}>", name),
    }
}

pub fn expr_eq(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Num(a), Expr::Num(b)) => a == b,
        (Expr::Str(a), Expr::Str(b)) => a == b,
        (Expr::Bool(a), Expr::Bool(b)) => a == b,
        (Expr::Nil, Expr::Nil) => true,
        (Expr::Sym(a), Expr::Sym(b)) => a == b,
        (Expr::List(a), Expr::List(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| expr_eq(x, y))
        }
        _ => false,
    }
}

pub fn expr_to_num(e: &Expr) -> Result<f64, String> {
    match e {
        Expr::Num(n) => Ok(*n),
        Expr::Str(s) => s
            .parse::<f64>()
            .map_err(|_| format!("Cannot convert '{}' to number", s)),
        Expr::Bool(true) => Ok(1.0),
        Expr::Bool(false) => Ok(0.0),
        Expr::Nil => Ok(0.0),
        _ => Err(format!("Cannot convert to number: {}", expr_to_string(e))),
    }
}

pub fn num(n: f64) -> Expr {
    Expr::Num(n)
}

pub fn int(n: i64) -> Expr {
    Expr::Num(n as f64)
}

pub fn string(s: impl Into<String>) -> Expr {
    Expr::Str(s.into())
}

pub fn sym(s: impl Into<String>) -> Expr {
    Expr::Sym(s.into())
}

pub fn list(items: Vec<Expr>) -> Expr {
    Expr::List(items)
}

pub fn nil() -> Expr {
    Expr::Nil
}

pub fn bool_val(b: bool) -> Expr {
    Expr::Bool(b)
}
