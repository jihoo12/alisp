use crate::expr::{Expr, expr_to_string};

fn skip_ws(s: &str, pos: &mut usize) {
    while *pos < s.len() && matches!(s.as_bytes()[*pos], b' ' | b'\n' | b'\r' | b'\t') {
        *pos += 1;
    }
}

fn json_parse_val(s: &str, pos: &mut usize) -> Result<Expr, String> {
    skip_ws(s, pos);
    if *pos >= s.len() {
        return Err("Unexpected end of JSON".into());
    }
    match s.as_bytes()[*pos] {
        b'"' => {
            *pos += 1;
            let mut result = String::new();
            while *pos < s.len() {
                match s.as_bytes()[*pos] {
                    b'"' => {
                        *pos += 1;
                        return Ok(Expr::Str(result));
                    }
                    b'\\' => {
                        *pos += 1;
                        if *pos >= s.len() {
                            return Err("Unterminated escape".into());
                        }
                        match s.as_bytes()[*pos] {
                            b'n' => result.push('\n'),
                            b't' => result.push('\t'),
                            b'r' => result.push('\r'),
                            b'\\' => result.push('\\'),
                            b'"' => result.push('"'),
                            b'/' => result.push('/'),
                            b'0' => result.push('\0'),
                            c => {
                                result.push('\\');
                                result.push(c as char);
                            }
                        }
                        *pos += 1;
                    }
                    c => {
                        result.push(c as char);
                        *pos += 1;
                    }
                }
            }
            Err("Unterminated JSON string".into())
        }
        b'[' => {
            *pos += 1;
            let mut arr = Vec::new();
            skip_ws(s, pos);
            if *pos < s.len() && s.as_bytes()[*pos] == b']' {
                *pos += 1;
                return Ok(Expr::List(arr));
            }
            loop {
                arr.push(json_parse_val(s, pos)?);
                skip_ws(s, pos);
                if *pos >= s.len() {
                    return Err("Unterminated JSON array".into());
                }
                match s.as_bytes()[*pos] {
                    b',' => *pos += 1,
                    b']' => {
                        *pos += 1;
                        return Ok(Expr::List(arr));
                    }
                    c => return Err(format!("Expected ',' or ']', got '{}'", c as char)),
                }
            }
        }
        b'{' => {
            *pos += 1;
            let mut pairs = Vec::new();
            skip_ws(s, pos);
            if *pos < s.len() && s.as_bytes()[*pos] == b'}' {
                *pos += 1;
                return Ok(Expr::List(pairs));
            }
            loop {
                let key = json_parse_val(s, pos)?;
                skip_ws(s, pos);
                if *pos >= s.len() || s.as_bytes()[*pos] != b':' {
                    return Err("Expected ':' in JSON object".into());
                }
                *pos += 1;
                let val = json_parse_val(s, pos)?;
                pairs.push(Expr::List(vec![key, val]));
                skip_ws(s, pos);
                if *pos >= s.len() {
                    return Err("Unterminated JSON object".into());
                }
                match s.as_bytes()[*pos] {
                    b',' => *pos += 1,
                    b'}' => {
                        *pos += 1;
                        return Ok(Expr::List(pairs));
                    }
                    c => return Err(format!("Expected ',' or '}}', got '{}'", c as char)),
                }
            }
        }
        b'-' | b'0'..=b'9' => {
            let start = *pos;
            if s.as_bytes()[*pos] == b'-' {
                *pos += 1;
            }
            while *pos < s.len() && s.as_bytes()[*pos].is_ascii_digit() {
                *pos += 1;
            }
            if *pos < s.len() && s.as_bytes()[*pos] == b'.' {
                *pos += 1;
                while *pos < s.len() && s.as_bytes()[*pos].is_ascii_digit() {
                    *pos += 1;
                }
            }
            if *pos < s.len() && matches!(s.as_bytes()[*pos], b'e' | b'E') {
                *pos += 1;
                if *pos < s.len() && matches!(s.as_bytes()[*pos], b'+' | b'-') {
                    *pos += 1;
                }
                while *pos < s.len() && s.as_bytes()[*pos].is_ascii_digit() {
                    *pos += 1;
                }
            }
            let num_str: String = s[start..*pos].chars().collect();
            let n: f64 = num_str
                .parse()
                .map_err(|e| format!("JSON number parse error: {}", e))?;
            Ok(Expr::Num(n))
        }
        b't' => {
            if s[*pos..].starts_with("true") {
                *pos += 4;
                Ok(Expr::Bool(true))
            } else {
                Err("Invalid JSON token".into())
            }
        }
        b'f' => {
            if s[*pos..].starts_with("false") {
                *pos += 5;
                Ok(Expr::Bool(false))
            } else {
                Err("Invalid JSON token".into())
            }
        }
        b'n' => {
            if s[*pos..].starts_with("null") {
                *pos += 4;
                Ok(Expr::Nil)
            } else {
                Err("Invalid JSON token".into())
            }
        }
        c => Err(format!("Unexpected character in JSON: '{}'", c as char)),
    }
}

pub fn json_parse_str(s: &str) -> Result<Expr, String> {
    let mut pos = 0;
    let result = json_parse_val(s, &mut pos)?;
    skip_ws(s, &mut pos);
    if pos < s.len() {
        return Err(format!("Trailing content after JSON at position {}", pos));
    }
    Ok(result)
}

pub fn json_is_object(e: &Expr) -> bool {
    if let Expr::List(v) = e {
        !v.is_empty()
            && v.iter().all(|item| {
                if let Expr::List(pair) = item {
                    pair.len() == 2 && matches!(&pair[0], Expr::Str(_))
                } else {
                    false
                }
            })
    } else {
        false
    }
}

fn json_stringify_val(e: &Expr, out: &mut String, compact: bool, indent: usize) {
    let pad = if compact {
        String::new()
    } else {
        "  ".repeat(indent)
    };
    match e {
        Expr::Num(n) => {
            if *n == (*n as i64) as f64 && n.is_finite() {
                out.push_str(&format!("{}", *n as i64));
            } else {
                out.push_str(&format!("{}", n));
            }
        }
        Expr::Str(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '\r' => out.push_str("\\r"),
                    c if c.is_control() => {
                        out.push_str(&format!("\\u{:04x}", c as u32));
                    }
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Expr::Bool(true) => out.push_str("true"),
        Expr::Bool(false) => out.push_str("false"),
        Expr::Nil => out.push_str("null"),
        Expr::List(v) => {
            if json_is_object(e) {
                out.push('{');
                for (i, item) in v.iter().enumerate() {
                    if let Expr::List(pair) = item {
                        if i > 0 {
                            out.push(',');
                        }
                        if !compact {
                            out.push('\n');
                            out.push_str(&"  ".repeat(indent + 1));
                        }
                        json_stringify_val(&pair[0], out, compact, indent + 1);
                        out.push_str(if compact { ":" } else { ": " });
                        json_stringify_val(&pair[1], out, compact, indent + 1);
                    }
                }
                if !v.is_empty() && !compact {
                    out.push('\n');
                    out.push_str(&pad);
                }
                out.push('}');
            } else {
                out.push('[');
                for (i, item) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    if !compact {
                        out.push('\n');
                        out.push_str(&"  ".repeat(indent + 1));
                    }
                    json_stringify_val(item, out, compact, indent + 1);
                }
                if !v.is_empty() && !compact {
                    out.push('\n');
                    out.push_str(&pad);
                }
                out.push(']');
            }
        }
        _ => {
            out.push_str(&format!("\"{}\"", expr_to_string(e)));
        }
    }
}

pub fn json_stringify(e: &Expr, compact: bool) -> String {
    let mut out = String::new();
    json_stringify_val(e, &mut out, compact, 0);
    out
}

use crate::Evaluator;
use crate::expr::Expr as E;

impl Evaluator {
    pub(crate) fn builtin_json_parse(&mut self, args: &[E]) -> Result<E, String> {
        if args.is_empty() { return Err("json-parse requires a string".into()); }
        let s = crate::expr::expr_to_string(&args[0]);
        json_parse_str(&s)
    }

    pub(crate) fn builtin_json_stringify(&mut self, args: &[E]) -> Result<E, String> {
        if args.is_empty() { return Err("json-stringify requires a value".into()); }
        let compact = if args.len() > 1 {
            match &args[1] {
                E::Bool(b) => *b,
                E::Sym(s) if s == "compact" => true,
                _ => false,
            }
        } else {
            false
        };
        Ok(E::Str(json_stringify(&args[0], compact)))
    }

    pub(crate) fn builtin_json_get(&mut self, args: &[E]) -> Result<E, String> {
        if args.len() < 2 { return Err("(json-get obj key)".into()); }
        let key = crate::expr::expr_to_string(&args[1]);
        match &args[0] {
            E::List(v) => {
                if json_is_object(&args[0]) {
                    for pair in v {
                        if let E::List(kvp) = pair {
                            if let E::Str(k) = &kvp[0] {
                                if k == &key { return Ok(kvp[1].clone()); }
                            }
                        }
                    }
                    Ok(E::Nil)
                } else {
                    if let Ok(idx) = key.parse::<usize>() {
                        Ok(v.get(idx).cloned().unwrap_or(E::Nil))
                    } else {
                        Err(format!("Key '{}' not found", key))
                    }
                }
            }
            E::Str(s) => {
                if let Ok(idx) = key.parse::<usize>() {
                    Ok(s.chars().nth(idx).map(|c| E::Str(c.to_string())).unwrap_or(E::Nil))
                } else {
                    Err("String index must be a number".into())
                }
            }
            _ => Err("json-get: first arg must be a list or string".into()),
        }
    }

    pub(crate) fn builtin_json_set(&mut self, args: &[E]) -> Result<E, String> {
        if args.len() < 3 { return Err("(json-set obj key value)".into()); }
        let key = crate::expr::expr_to_string(&args[1]);
        let val = args[2].clone();
        match &args[0] {
            E::List(_) => self.builtin_assoc(&[args[0].clone(), E::Str(key), val]),
            _ => Err("json-set: first arg must be a list (object)".into()),
        }
    }

    pub(crate) fn builtin_json_keys(&mut self, args: &[E]) -> Result<E, String> {
        self.builtin_keys(args)
    }
}
