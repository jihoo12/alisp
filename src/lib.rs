//! alisp - A tiny Lisp interpreter for AI agents.
//!
//! Zero-dependency Rust crate providing shell execution, file I/O, HTTP,
//! JSON handling, string/list operations, and error handling.
//!
//! # Quick Start
//!
//! ```rust
//! use alisp::Evaluator;
//!
//! let mut eval = Evaluator::new();
//! let result = eval.eval_str("(+ 1 2)").unwrap().unwrap();
//! assert_eq!(alisp::expr_to_string(&result), "3");
//! ```

#![allow(dead_code)]

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::Command;

pub const VERSION: &str = "0.1.0";

// ===================== TOKENIZER =====================

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    Number(f64),
    Str(String),
    Sym(String),
    Nil,
    Bool(bool),
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < len {
        match chars[i] {
            ';' => {
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '"' => {
                i += 1;
                let mut s = String::new();
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < len {
                        i += 1;
                        match chars[i] {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            '\\' => s.push('\\'),
                            '"' => s.push('"'),
                            '0' => s.push('\0'),
                            c => {
                                s.push('\\');
                                s.push(c);
                            }
                        }
                    } else {
                        s.push(chars[i]);
                    }
                    i += 1;
                }
                if i >= len {
                    return Err("Unterminated string".into());
                }
                i += 1;
                tokens.push(Token::Str(s));
            }
            c if c.is_whitespace() => {
                i += 1;
            }
            _ => {
                let start = i;
                while i < len {
                    let c = chars[i];
                    if c.is_whitespace() || c == '(' || c == ')' || c == '"' || c == ';' {
                        break;
                    }
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                match word.as_str() {
                    "nil" | "null" => tokens.push(Token::Nil),
                    "true" | "t" => tokens.push(Token::Bool(true)),
                    "false" | "f" => tokens.push(Token::Bool(false)),
                    _ => {
                        if let Ok(n) = word.parse::<f64>() {
                            tokens.push(Token::Number(n));
                        } else {
                            tokens.push(Token::Sym(word));
                        }
                    }
                }
            }
        }
    }
    Ok(tokens)
}

// ===================== AST =====================

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

/// Construct Expr helpers for building ASTs from Rust.

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

// ===================== PARSER =====================

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Token::LParen) => {
                let mut list = Vec::new();
                loop {
                    match self.peek() {
                        Some(Token::RParen) => {
                            self.next();
                            return Ok(Expr::List(list));
                        }
                        None => return Err("Unexpected EOF in list".into()),
                        _ => list.push(self.parse_expr()?),
                    }
                }
            }
            Some(Token::Number(n)) => Ok(Expr::Num(n)),
            Some(Token::Str(s)) => Ok(Expr::Str(s)),
            Some(Token::Bool(b)) => Ok(Expr::Bool(b)),
            Some(Token::Nil) => Ok(Expr::Nil),
            Some(Token::Sym(s)) => Ok(Expr::Sym(s)),
            Some(Token::RParen) => Err("Unexpected ')'".into()),
            None => Err("Unexpected end of input".into()),
        }
    }

    fn parse_all(&mut self) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        while self.pos < self.tokens.len() {
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }
}

/// Parse a string into a list of expressions.
pub fn parse(input: &str) -> Result<Vec<Expr>, String> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_all()
}

/// Parse a single expression from a string.
pub fn parse_first(input: &str) -> Result<Expr, String> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_expr()
}

/// Count unmatched open parentheses (for multiline REPL input).
pub fn count_parens(s: &str) -> i32 {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut in_comment = false;
    for c in s.chars() {
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            continue;
        }
        if c == ';' && !in_str {
            in_comment = true;
            continue;
        }
        if esc {
            esc = false;
            continue;
        }
        if c == '\\' && in_str {
            esc = true;
            continue;
        }
        if c == '"' {
            in_str = !in_str;
            continue;
        }
        if !in_str {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
    }
    depth
}

// ===================== JSON =====================

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

/// Parse a JSON string into an Expr.
pub fn json_parse_str(s: &str) -> Result<Expr, String> {
    let mut pos = 0;
    let result = json_parse_val(s, &mut pos)?;
    skip_ws(s, &mut pos);
    if pos < s.len() {
        return Err(format!("Trailing content after JSON at position {}", pos));
    }
    Ok(result)
}

fn json_is_object(e: &Expr) -> bool {
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

/// Serialize an Expr to a JSON string.
pub fn json_stringify(e: &Expr, compact: bool) -> String {
    let mut out = String::new();
    json_stringify_val(e, &mut out, compact, 0);
    out
}

// ===================== EVALUATOR =====================

pub struct Evaluator {
    globals: HashMap<String, Expr>,
    locals: Vec<HashMap<String, Expr>>,
    start_time: std::time::Instant,
}

impl Evaluator {
    pub fn new() -> Self {
        let mut globals = HashMap::new();
        let builtins: &[&str] = &[
            "exec", "shell", "sh", "exec-result",
            "read", "read-file", "write", "write-file", "append", "append-file",
            "exists", "file-exists", "rm", "delete", "mkdir", "cp", "copy", "mv", "move",
            "ls", "list-dir", "glob", "cwd", "pwd", "cd",
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

    /// Parse and evaluate a code string. Returns the last expression's value.
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

    /// Evaluate a file and return the last expression's value.
    pub fn eval_file(&mut self, path: &str) -> Result<Option<Expr>, String> {
        let code = fs::read_to_string(path).map_err(|e| format!("read '{}': {}", path, e))?;
        self.eval_str(&code)
    }

    fn eval_list(&mut self, list: &[Expr]) -> Result<Expr, String> {
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

    fn call(&mut self, func: &Expr, args: &[Expr]) -> Result<Expr, String> {
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

    // ===================== BUILTINS =====================

    fn call_builtin(&mut self, name: &str, args: &[Expr]) -> Result<Expr, String> {
        match name {
            "exec" | "shell" | "sh" => self.builtin_exec(args, false),
            "exec-result" => self.builtin_exec(args, true),
            "read" | "read-file" => self.builtin_read(args),
            "write" | "write-file" => self.builtin_write(args),
            "append" | "append-file" => self.builtin_append(args),
            "exists" | "file-exists" => self.builtin_exists(args),
            "rm" | "delete" => self.builtin_rm(args),
            "mkdir" => self.builtin_mkdir(args),
            "cp" | "copy" => self.builtin_cp(args),
            "mv" | "move" => self.builtin_mv(args),
            "ls" | "list-dir" => self.builtin_ls(args),
            "glob" => self.builtin_glob(args),
            "cwd" | "pwd" => self.builtin_cwd(args),
            "cd" => self.builtin_cd(args),
            "getenv" | "env-get" => self.builtin_getenv(args),
            "setenv" | "env-set" => self.builtin_setenv(args),
            "env" => self.builtin_env(args),
            "str" => self.builtin_str(args),
            "split" => self.builtin_split(args),
            "join" => self.builtin_join(args),
            "trim" => self.builtin_trim(args),
            "contains" | "includes" => self.builtin_contains(args),
            "starts-with" => self.builtin_starts_with(args),
            "ends-with" => self.builtin_ends_with(args),
            "replace" => self.builtin_replace(args),
            "upper" => self.builtin_upper(args),
            "lower" => self.builtin_lower(args),
            "substr" => self.builtin_substr(args),
            "find" => self.builtin_find(args),
            "format" => self.builtin_format(args),
            "list" => self.builtin_list_fn(args),
            "car" | "head" | "first" => self.builtin_car(args),
            "cdr" | "tail" | "rest" => self.builtin_cdr(args),
            "cons" => self.builtin_cons(args),
            "len" | "length" | "size" => self.builtin_len(args),
            "push" => self.builtin_push(args),
            "nth" | "at" => self.builtin_nth(args),
            "map" => self.builtin_map(args),
            "filter" | "select" => self.builtin_filter(args),
            "reduce" | "fold" => self.builtin_reduce(args),
            "each" | "for-each" => self.builtin_each(args),
            "range" => self.builtin_range(args),
            "reverse" => self.builtin_reverse(args),
            "sort" => self.builtin_sort(args),
            "flatten" => self.builtin_flatten(args),
            "last" => self.builtin_last(args),
            "empty?" => self.builtin_empty(args),
            "any" => self.builtin_any(args),
            "all" => self.builtin_all(args),
            "zip" => self.builtin_zip(args),
            "assoc" => self.builtin_assoc(args),
            "dissoc" => self.builtin_dissoc(args),
            "keys" => self.builtin_keys(args),
            "values" => self.builtin_values(args),
            "merge" => self.builtin_merge(args),
            "+" => self.builtin_add(args),
            "-" => self.builtin_sub(args),
            "*" => self.builtin_mul(args),
            "/" => self.builtin_div(args),
            "%" | "mod" => self.builtin_mod(args),
            "pow" => self.builtin_pow(args),
            "sqrt" => self.builtin_sqrt(args),
            "abs" => self.builtin_abs(args),
            "min" => self.builtin_min(args),
            "max" => self.builtin_max(args),
            "floor" => self.builtin_floor_fn(args),
            "ceil" => self.builtin_ceil_fn(args),
            "round" => self.builtin_round_fn(args),
            "rand" | "random" => self.builtin_rand(args),
            "inc" => self.builtin_inc(args),
            "dec" => self.builtin_dec(args),
            "=" | "==" => self.builtin_eq(args),
            "!=" => self.builtin_ne(args),
            "<" => self.builtin_lt(args),
            ">" => self.builtin_gt(args),
            "<=" => self.builtin_lte(args),
            ">=" => self.builtin_gte(args),
            "not" => self.builtin_not(args),
            "type" | "type-of" => self.builtin_type(args),
            "int" => self.builtin_int(args),
            "float" => self.builtin_float(args),
            "number?" => self.builtin_is_number(args),
            "string?" => self.builtin_is_string(args),
            "list?" => self.builtin_is_list(args),
            "nil?" => self.builtin_is_nil(args),
            "bool?" => self.builtin_is_bool(args),
            "print" => self.builtin_print(args, false),
            "println" => self.builtin_print(args, true),
            "eprint" => self.builtin_eprint(args, false),
            "eprintln" => self.builtin_eprint(args, true),
            "input" => self.builtin_input(args),
            "http-get" => self.builtin_http(args, "GET"),
            "http-post" => self.builtin_http_with_body(args, "POST"),
            "http-put" => self.builtin_http_with_body(args, "PUT"),
            "http-delete" => self.builtin_http(args, "DELETE"),
            "http" => self.builtin_http_request(args),
            "json-parse" | "json" => self.builtin_json_parse(args),
            "json-stringify" | "json-str" => self.builtin_json_stringify(args),
            "json-get" | "jget" => self.builtin_json_get(args),
            "json-set" | "jset" => self.builtin_json_set(args),
            "json-keys" => self.builtin_json_keys(args),
            "sleep" => self.builtin_sleep(args),
            "time" => self.builtin_time(args),
            "timestamp" => self.builtin_timestamp(args),
            "exit" | "quit" => self.builtin_exit(args),
            _ => Err(format!("Unknown function: {}", name)),
        }
    }

    // ---- Shell ----

    fn builtin_exec(&mut self, args: &[Expr], return_result: bool) -> Result<Expr, String> {
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

    // ---- File I/O ----

    fn builtin_read(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("read requires a filename".into());
        }
        let path = expr_to_string(&args[0]);
        let content = fs::read_to_string(&path).map_err(|e| format!("read '{}': {}", path, e))?;
        Ok(Expr::Str(content))
    }

    fn builtin_write(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(write path content)".into());
        }
        let path = expr_to_string(&args[0]);
        let content = expr_to_string(&args[1]);
        fs::write(&path, &content).map_err(|e| format!("write '{}': {}", path, e))?;
        Ok(Expr::Bool(true))
    }

    fn builtin_append(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(append path content)".into());
        }
        let path = expr_to_string(&args[0]);
        let content = expr_to_string(&args[1]);
        use std::fs::OpenOptions;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("append '{}': {}", path, e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("append '{}': {}", path, e))?;
        Ok(Expr::Bool(true))
    }

    fn builtin_exists(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("exists requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        Ok(Expr::Bool(std::path::Path::new(&path).exists()))
    }

    fn builtin_rm(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("rm requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        if std::path::Path::new(&path).is_dir() {
            fs::remove_dir_all(&path).map_err(|e| format!("rm '{}': {}", path, e))?;
        } else {
            fs::remove_file(&path).map_err(|e| format!("rm '{}': {}", path, e))?;
        }
        Ok(Expr::Bool(true))
    }

    fn builtin_mkdir(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("mkdir requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        fs::create_dir_all(&path).map_err(|e| format!("mkdir '{}': {}", path, e))?;
        Ok(Expr::Bool(true))
    }

    fn builtin_cp(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(cp src dst)".into());
        }
        let src = expr_to_string(&args[0]);
        let dst = expr_to_string(&args[1]);
        fs::copy(&src, &dst).map_err(|e| format!("cp '{}' '{}': {}", src, dst, e))?;
        Ok(Expr::Bool(true))
    }

    fn builtin_mv(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(mv src dst)".into());
        }
        let src = expr_to_string(&args[0]);
        let dst = expr_to_string(&args[1]);
        fs::rename(&src, &dst).map_err(|e| format!("mv '{}' '{}': {}", src, dst, e))?;
        Ok(Expr::Bool(true))
    }

    fn builtin_ls(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let path = if args.is_empty() {
            ".".to_string()
        } else {
            expr_to_string(&args[0])
        };
        let entries = fs::read_dir(&path).map_err(|e| format!("ls '{}': {}", path, e))?;
        let mut names: Vec<Expr> = entries
            .filter_map(|e| e.ok())
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                Expr::Str(name)
            })
            .collect();
        names.sort_by(|a, b| expr_to_string(a).cmp(&expr_to_string(b)));
        Ok(Expr::List(names))
    }

    fn builtin_glob(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("glob requires a pattern".into());
        }
        let pattern = expr_to_string(&args[0]);
        let entries = glob_simple(&pattern);
        Ok(Expr::List(
            entries.into_iter().map(Expr::Str).collect(),
        ))
    }

    fn builtin_cwd(&mut self, _args: &[Expr]) -> Result<Expr, String> {
        let cwd = env::current_dir().map_err(|e| format!("cwd: {}", e))?;
        Ok(Expr::Str(cwd.to_string_lossy().to_string()))
    }

    fn builtin_cd(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("cd requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        env::set_current_dir(&path).map_err(|e| format!("cd '{}': {}", path, e))?;
        Ok(Expr::Bool(true))
    }

    // ---- Environment ----

    fn builtin_getenv(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("getenv requires a name".into());
        }
        let name = expr_to_string(&args[0]);
        Ok(match env::var(&name) {
            Ok(v) => Expr::Str(v),
            Err(_) => Expr::Nil,
        })
    }

    fn builtin_setenv(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(setenv name value)".into());
        }
        let name = expr_to_string(&args[0]);
        let value = expr_to_string(&args[1]);
        // SAFETY: We're setting an environment variable. This is safe in
        // single-threaded contexts and the value comes from user input.
        unsafe { env::set_var(&name, &value) };
        Ok(Expr::Bool(true))
    }

    fn builtin_env(&mut self, _args: &[Expr]) -> Result<Expr, String> {
        let pairs: Vec<Expr> = env::vars()
            .map(|(k, v)| Expr::List(vec![Expr::Str(k), Expr::Str(v)]))
            .collect();
        Ok(Expr::List(pairs))
    }

    // ---- String Ops ----

    fn builtin_str(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let result: String = args.iter().map(expr_to_string).collect();
        Ok(Expr::Str(result))
    }

    fn builtin_split(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(split string delimiter)".into());
        }
        let s = expr_to_string(&args[0]);
        let delim = expr_to_string(&args[1]);
        let parts: Vec<Expr> = s.split(&*delim).map(|s| Expr::Str(s.to_string())).collect();
        Ok(Expr::List(parts))
    }

    fn builtin_join(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    fn builtin_trim(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("trim requires a string".into());
        }
        Ok(Expr::Str(expr_to_string(&args[0]).trim().to_string()))
    }

    fn builtin_contains(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(contains haystack needle)".into());
        }
        let haystack = expr_to_string(&args[0]);
        let needle = expr_to_string(&args[1]);
        Ok(Expr::Bool(haystack.contains(&*needle)))
    }

    fn builtin_starts_with(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(starts-with s prefix)".into());
        }
        let s = expr_to_string(&args[0]);
        let prefix = expr_to_string(&args[1]);
        Ok(Expr::Bool(s.starts_with(&*prefix)))
    }

    fn builtin_ends_with(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(ends-with s suffix)".into());
        }
        let s = expr_to_string(&args[0]);
        let suffix = expr_to_string(&args[1]);
        Ok(Expr::Bool(s.ends_with(&*suffix)))
    }

    fn builtin_replace(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 {
            return Err("(replace s old new)".into());
        }
        let s = expr_to_string(&args[0]);
        let old = expr_to_string(&args[1]);
        let new = expr_to_string(&args[2]);
        Ok(Expr::Str(s.replace(&*old, &*new)))
    }

    fn builtin_upper(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("upper requires a string".into());
        }
        Ok(Expr::Str(expr_to_string(&args[0]).to_uppercase()))
    }

    fn builtin_lower(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("lower requires a string".into());
        }
        Ok(Expr::Str(expr_to_string(&args[0]).to_lowercase()))
    }

    fn builtin_substr(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    fn builtin_find(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    fn builtin_format(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    // ---- List Ops ----

    fn builtin_list_fn(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::List(args.to_vec()))
    }

    fn builtin_car(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("car requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => Ok(v.first().cloned().unwrap_or(Expr::Nil)),
            Expr::Str(s) => Ok(s
                .chars()
                .next()
                .map(|c| Expr::Str(c.to_string()))
                .unwrap_or(Expr::Nil)),
            _ => Err("car: argument must be a list or string".into()),
        }
    }

    fn builtin_cdr(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("cdr requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => Ok(Expr::List(v[1..].to_vec())),
            Expr::Str(s) => Ok(Expr::Str(s.chars().skip(1).collect())),
            _ => Err("cdr: argument must be a list or string".into()),
        }
    }

    fn builtin_cons(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(cons elem list)".into());
        }
        match &args[1] {
            Expr::List(v) => {
                let mut result = vec![args[0].clone()];
                result.extend_from_slice(v);
                Ok(Expr::List(result))
            }
            Expr::Nil => Ok(Expr::List(vec![args[0].clone()])),
            _ => Err("cons: second arg must be a list".into()),
        }
    }

    fn builtin_len(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("len requires an argument".into());
        }
        let n = match &args[0] {
            Expr::List(v) => v.len(),
            Expr::Str(s) => s.len(),
            _ => return Err("len: argument must be a list or string".into()),
        };
        Ok(Expr::Num(n as f64))
    }

    fn builtin_push(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(push list elem)".into());
        }
        match &args[0] {
            Expr::List(v) => {
                let mut result = v.clone();
                result.push(args[1].clone());
                Ok(Expr::List(result))
            }
            _ => Err("push: first arg must be a list".into()),
        }
    }

    fn builtin_nth(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(nth list index)".into());
        }
        let idx = match &args[1] {
            Expr::Num(n) => *n as usize,
            _ => return Err("nth: index must be a number".into()),
        };
        match &args[0] {
            Expr::List(v) => Ok(v.get(idx).cloned().unwrap_or(Expr::Nil)),
            Expr::Str(s) => Ok(s
                .chars()
                .nth(idx)
                .map(|c| Expr::Str(c.to_string()))
                .unwrap_or(Expr::Nil)),
            _ => Err("nth: first arg must be a list or string".into()),
        }
    }

    fn builtin_map(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(map func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v,
            _ => return Err("map: second arg must be a list".into()),
        };
        let mut result = Vec::with_capacity(items.len());
        for item in items {
            let mapped = self.call(func, &[item.clone()])?;
            result.push(mapped);
        }
        Ok(Expr::List(result))
    }

    fn builtin_filter(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(filter func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v,
            _ => return Err("filter: second arg must be a list".into()),
        };
        let mut result = Vec::new();
        for item in items {
            if is_truthy(&self.call(func, &[item.clone()])?) {
                result.push(item.clone());
            }
        }
        Ok(Expr::List(result))
    }

    fn builtin_reduce(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 {
            return Err("(reduce func init list)".into());
        }
        let func = &args[0];
        let mut acc = args[1].clone();
        let items = match &args[2] {
            Expr::List(v) => v,
            _ => return Err("reduce: third arg must be a list".into()),
        };
        for item in items {
            acc = self.call(func, &[acc, item.clone()])?;
        }
        Ok(acc)
    }

    fn builtin_each(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(each func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v.clone(),
            _ => return Err("each: second arg must be a list".into()),
        };
        for item in &items {
            self.call(func, &[item.clone()])?;
        }
        Ok(Expr::Nil)
    }

    fn builtin_range(&mut self, args: &[Expr]) -> Result<Expr, String> {
        match args.len() {
            1 => {
                let end = match &args[0] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: argument must be a number".into()),
                };
                let start: i64 = if end > 0 { 0 } else { end };
                let step: i64 = if end > 0 { 1 } else { -1 };
                let items: Vec<Expr> = (start..end)
                    .step_by(step.unsigned_abs() as usize)
                    .map(|i| Expr::Num(i as f64))
                    .collect();
                Ok(Expr::List(items))
            }
            2 => {
                let start = match &args[0] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                let end = match &args[1] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                let step: i64 = if end > start { 1 } else { -1 };
                let items: Vec<Expr> = (start..end)
                    .step_by(step.unsigned_abs() as usize)
                    .map(|i| Expr::Num(i as f64))
                    .collect();
                Ok(Expr::List(items))
            }
            3 => {
                let start = match &args[0] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                let end = match &args[1] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                let step = match &args[2] {
                    Expr::Num(n) => *n as i64,
                    _ => return Err("range: arguments must be numbers".into()),
                };
                if step == 0 {
                    return Err("range: step cannot be zero".into());
                }
                let items: Vec<Expr> = if step > 0 {
                    (start..end).step_by(step as usize).map(|i| Expr::Num(i as f64)).collect()
                } else {
                    (end..=start).rev().step_by((-step) as usize).map(|i| Expr::Num(i as f64)).collect()
                };
                Ok(Expr::List(items))
            }
            _ => Err("(range end) or (range start end) or (range start end step)".into()),
        }
    }

    fn builtin_reverse(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("reverse requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => {
                let mut result = v.clone();
                result.reverse();
                Ok(Expr::List(result))
            }
            Expr::Str(s) => Ok(Expr::Str(s.chars().rev().collect())),
            _ => Err("reverse: argument must be a list or string".into()),
        }
    }

    fn builtin_sort(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("sort requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => {
                let mut result = v.clone();
                result.sort_by(|a, b| {
                    expr_to_string(a).cmp(&expr_to_string(b))
                });
                Ok(Expr::List(result))
            }
            _ => Err("sort: argument must be a list".into()),
        }
    }

    fn builtin_flatten(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("flatten requires a list".into());
        }
        fn flatten_list(e: &Expr) -> Vec<Expr> {
            match e {
                Expr::List(v) => v.iter().flat_map(flatten_list).collect(),
                other => vec![other.clone()],
            }
        }
        Ok(Expr::List(flatten_list(&args[0])))
    }

    fn builtin_last(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("last requires a list".into());
        }
        match &args[0] {
            Expr::List(v) => Ok(v.last().cloned().unwrap_or(Expr::Nil)),
            Expr::Str(s) => Ok(s
                .chars()
                .last()
                .map(|c| Expr::Str(c.to_string()))
                .unwrap_or(Expr::Nil)),
            _ => Err("last: argument must be a list or string".into()),
        }
    }

    fn builtin_empty(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("empty? requires an argument".into());
        }
        let empty = match &args[0] {
            Expr::List(v) => v.is_empty(),
            Expr::Str(s) => s.is_empty(),
            Expr::Nil => true,
            _ => false,
        };
        Ok(Expr::Bool(empty))
    }

    fn builtin_any(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(any func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v,
            _ => return Err("any: second arg must be a list".into()),
        };
        for item in items {
            if is_truthy(&self.call(func, &[item.clone()])?) {
                return Ok(Expr::Bool(true));
            }
        }
        Ok(Expr::Bool(false))
    }

    fn builtin_all(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(all func list)".into());
        }
        let func = &args[0];
        let items = match &args[1] {
            Expr::List(v) => v,
            _ => return Err("all: second arg must be a list".into()),
        };
        for item in items {
            if !is_truthy(&self.call(func, &[item.clone()])?) {
                return Ok(Expr::Bool(false));
            }
        }
        Ok(Expr::Bool(true))
    }

    fn builtin_zip(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("zip requires at least one list".into());
        }
        let lists: Result<Vec<Vec<Expr>>, String> = args
            .iter()
            .map(|a| match a {
                Expr::List(v) => Ok(v.clone()),
                _ => Err("zip: all arguments must be lists".into()),
            })
            .collect();
        let lists = lists?;
        let min_len = lists.iter().map(|l| l.len()).min().unwrap_or(0);
        let result: Vec<Expr> = (0..min_len)
            .map(|i| Expr::List(lists.iter().map(|l| l[i].clone()).collect()))
            .collect();
        Ok(Expr::List(result))
    }

    // ---- Assoc helpers ----

    fn builtin_assoc(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 {
            return Err("(assoc alist key value)".into());
        }
        let key = expr_to_string(&args[1]);
        let val = args[2].clone();
        let mut pairs = match &args[0] {
            Expr::List(v) => v.clone(),
            Expr::Nil => Vec::new(),
            _ => return Err("assoc: first arg must be a list".into()),
        };
        pairs.retain(|p| {
            if let Expr::List(pair) = p {
                if let Expr::Str(k) = &pair[0] {
                    return k != &key;
                }
            }
            true
        });
        pairs.push(Expr::List(vec![Expr::Str(key), val]));
        Ok(Expr::List(pairs))
    }

    fn builtin_dissoc(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(dissoc alist key)".into());
        }
        let key = expr_to_string(&args[1]);
        let mut pairs = match &args[0] {
            Expr::List(v) => v.clone(),
            _ => return Err("dissoc: first arg must be a list".into()),
        };
        pairs.retain(|p| {
            if let Expr::List(pair) = p {
                if let Expr::Str(k) = &pair[0] {
                    return k != &key;
                }
            }
            true
        });
        Ok(Expr::List(pairs))
    }

    fn builtin_keys(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("keys requires an object".into());
        }
        match &args[0] {
            Expr::List(v) => {
                if json_is_object(&args[0]) {
                    let keys: Vec<Expr> = v
                        .iter()
                        .filter_map(|p| {
                            if let Expr::List(pair) = p {
                                Some(pair[0].clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    Ok(Expr::List(keys))
                } else {
                    Ok(Expr::List(Vec::new()))
                }
            }
            _ => Err("keys: argument must be an object (list of pairs)".into()),
        }
    }

    fn builtin_values(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("values requires an object".into());
        }
        match &args[0] {
            Expr::List(v) => {
                if json_is_object(&args[0]) {
                    let vals: Vec<Expr> = v
                        .iter()
                        .filter_map(|p| {
                            if let Expr::List(pair) = p {
                                Some(pair[1].clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    Ok(Expr::List(vals))
                } else {
                    Ok(args[0].clone())
                }
            }
            _ => Err("values: argument must be an object (list of pairs)".into()),
        }
    }

    fn builtin_merge(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let mut result = Vec::new();
        for arg in args {
            if let Expr::List(v) = arg {
                result.extend_from_slice(v);
            }
        }
        Ok(Expr::List(result))
    }

    // ---- Arithmetic ----

    fn num_args(&self, args: &[Expr]) -> Result<Vec<f64>, String> {
        args.iter()
            .map(|a| expr_to_num(a))
            .collect()
    }

    fn builtin_add(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    fn builtin_sub(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("- requires at least one argument".into());
        }
        if nums.len() == 1 {
            return Ok(Expr::Num(-nums[0]));
        }
        Ok(Expr::Num(nums[1..].iter().fold(nums[0], |acc, n| acc - n)))
    }

    fn builtin_mul(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        Ok(Expr::Num(nums.iter().product()))
    }

    fn builtin_div(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    fn builtin_mod(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.len() < 2 {
            return Err("% requires at least two arguments".into());
        }
        if nums[1] == 0.0 {
            return Err("Modulo by zero".into());
        }
        Ok(Expr::Num(nums[0] % nums[1]))
    }

    fn builtin_pow(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.len() < 2 {
            return Err("pow requires two arguments".into());
        }
        Ok(Expr::Num(nums[0].powf(nums[1])))
    }

    fn builtin_sqrt(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("sqrt requires an argument".into());
        }
        Ok(Expr::Num(nums[0].sqrt()))
    }

    fn builtin_abs(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("abs requires an argument".into());
        }
        Ok(Expr::Num(nums[0].abs()))
    }

    fn builtin_min(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("min requires arguments".into());
        }
        Ok(Expr::Num(nums.iter().cloned().fold(f64::INFINITY, f64::min)))
    }

    fn builtin_max(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("max requires arguments".into());
        }
        Ok(Expr::Num(nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)))
    }

    fn builtin_floor_fn(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("floor requires an argument".into());
        }
        Ok(Expr::Num(nums[0].floor()))
    }

    fn builtin_ceil_fn(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("ceil requires an argument".into());
        }
        Ok(Expr::Num(nums[0].ceil()))
    }

    fn builtin_round_fn(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Err("round requires an argument".into());
        }
        Ok(Expr::Num(nums[0].round()))
    }

    fn builtin_rand(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            Ok(Expr::Num(rand_f64()))
        } else {
            let nums = self.num_args(args)?;
            let max = nums[0];
            Ok(Expr::Num((rand_f64() * max).floor()))
        }
    }

    fn builtin_inc(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Ok(Expr::Num(1.0));
        }
        Ok(Expr::Num(nums[0] + 1.0))
    }

    fn builtin_dec(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let nums = self.num_args(args)?;
        if nums.is_empty() {
            return Ok(Expr::Num(-1.0));
        }
        Ok(Expr::Num(nums[0] - 1.0))
    }

    // ---- Comparison ----

    fn builtin_eq(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("= requires two arguments".into());
        }
        Ok(Expr::Bool(expr_eq(&args[0], &args[1])))
    }

    fn builtin_ne(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("!= requires two arguments".into());
        }
        Ok(Expr::Bool(!expr_eq(&args[0], &args[1])))
    }

    fn builtin_lt(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 { return Err("< requires two arguments".into()); }
        let a = expr_to_num(&args[0])?;
        let b = expr_to_num(&args[1])?;
        Ok(Expr::Bool(a < b))
    }

    fn builtin_gt(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 { return Err("> requires two arguments".into()); }
        let a = expr_to_num(&args[0])?;
        let b = expr_to_num(&args[1])?;
        Ok(Expr::Bool(a > b))
    }

    fn builtin_lte(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 { return Err("<= requires two arguments".into()); }
        let a = expr_to_num(&args[0])?;
        let b = expr_to_num(&args[1])?;
        Ok(Expr::Bool(a <= b))
    }

    fn builtin_gte(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 { return Err(">= requires two arguments".into()); }
        let a = expr_to_num(&args[0])?;
        let b = expr_to_num(&args[1])?;
        Ok(Expr::Bool(a >= b))
    }

    // ---- Logic ----

    fn builtin_not(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("not requires an argument".into());
        }
        Ok(Expr::Bool(!is_truthy(&args[0])))
    }

    // ---- Type ----

    fn builtin_type(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    fn builtin_int(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    fn builtin_float(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    fn builtin_is_number(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(false, |a| matches!(a, Expr::Num(_)))))
    }
    fn builtin_is_string(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(false, |a| matches!(a, Expr::Str(_)))))
    }
    fn builtin_is_list(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(false, |a| matches!(a, Expr::List(_)))))
    }
    fn builtin_is_nil(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(true, |a| matches!(a, Expr::Nil))))
    }
    fn builtin_is_bool(&mut self, args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Bool(args.first().map_or(false, |a| matches!(a, Expr::Bool(_)))))
    }

    // ---- IO ----

    fn builtin_print(&mut self, args: &[Expr], newline: bool) -> Result<Expr, String> {
        let s: String = args.iter().map(expr_to_string).collect::<Vec<_>>().join(" ");
        if newline {
            println!("{}", s);
        } else {
            print!("{}", s);
            io::stdout().flush().ok();
        }
        Ok(Expr::Nil)
    }

    fn builtin_eprint(&mut self, args: &[Expr], newline: bool) -> Result<Expr, String> {
        let s: String = args.iter().map(expr_to_string).collect::<Vec<_>>().join(" ");
        if newline {
            eprintln!("{}", s);
        } else {
            eprint!("{}", s);
            io::stderr().flush().ok();
        }
        Ok(Expr::Nil)
    }

    fn builtin_input(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    // ---- HTTP ----

    fn builtin_http(&mut self, args: &[Expr], method: &str) -> Result<Expr, String> {
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

    fn builtin_http_with_body(&mut self, args: &[Expr], method: &str) -> Result<Expr, String> {
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

    fn builtin_http_request(&mut self, args: &[Expr]) -> Result<Expr, String> {
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

    // ---- JSON ----

    fn builtin_json_parse(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() { return Err("json-parse requires a string".into()); }
        let s = expr_to_string(&args[0]);
        json_parse_str(&s)
    }

    fn builtin_json_stringify(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() { return Err("json-stringify requires a value".into()); }
        let compact = if args.len() > 1 {
            match &args[1] {
                Expr::Bool(b) => *b,
                Expr::Sym(s) if s == "compact" => true,
                _ => false,
            }
        } else {
            false
        };
        Ok(Expr::Str(json_stringify(&args[0], compact)))
    }

    fn builtin_json_get(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 { return Err("(json-get obj key)".into()); }
        let key = expr_to_string(&args[1]);
        match &args[0] {
            Expr::List(v) => {
                if json_is_object(&args[0]) {
                    for pair in v {
                        if let Expr::List(kvp) = pair {
                            if let Expr::Str(k) = &kvp[0] {
                                if k == &key { return Ok(kvp[1].clone()); }
                            }
                        }
                    }
                    Ok(Expr::Nil)
                } else {
                    if let Ok(idx) = key.parse::<usize>() {
                        Ok(v.get(idx).cloned().unwrap_or(Expr::Nil))
                    } else {
                        Err(format!("Key '{}' not found", key))
                    }
                }
            }
            Expr::Str(s) => {
                if let Ok(idx) = key.parse::<usize>() {
                    Ok(s.chars().nth(idx).map(|c| Expr::Str(c.to_string())).unwrap_or(Expr::Nil))
                } else {
                    Err("String index must be a number".into())
                }
            }
            _ => Err("json-get: first arg must be a list or string".into()),
        }
    }

    fn builtin_json_set(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 { return Err("(json-set obj key value)".into()); }
        let key = expr_to_string(&args[1]);
        let val = args[2].clone();
        match &args[0] {
            Expr::List(_) => self.builtin_assoc(&[args[0].clone(), Expr::Str(key), val]),
            _ => Err("json-set: first arg must be a list (object)".into()),
        }
    }

    fn builtin_json_keys(&mut self, args: &[Expr]) -> Result<Expr, String> {
        self.builtin_keys(args)
    }

    // ---- Misc ----

    fn builtin_sleep(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() { return Err("sleep requires duration in seconds".into()); }
        let secs = match &args[0] {
            Expr::Num(n) => *n,
            _ => return Err("sleep: argument must be a number".into()),
        };
        std::thread::sleep(std::time::Duration::from_secs_f64(secs));
        Ok(Expr::Nil)
    }

    fn builtin_time(&mut self, _args: &[Expr]) -> Result<Expr, String> {
        Ok(Expr::Num(self.start_time.elapsed().as_secs_f64()))
    }

    fn builtin_timestamp(&mut self, _args: &[Expr]) -> Result<Expr, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("timestamp: {}", e))?;
        Ok(Expr::Num(now.as_secs_f64()))
    }

    fn builtin_exit(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let code = if args.is_empty() { 0 } else {
            match &args[0] {
                Expr::Num(n) => *n as i32,
                _ => 0,
            }
        };
        std::process::exit(code);
    }
}

// ---- Helpers ----

fn glob_simple(pattern: &str) -> Vec<String> {
    let mut results = Vec::new();
    if let Some(pos) = pattern.rfind('/') {
        let dir = &pattern[..pos];
        let file_pattern = &pattern[pos + 1..];
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if glob_match(file_pattern, &name) {
                    results.push(format!("{}/{}", dir, name));
                }
            }
        }
    } else if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if glob_match(pattern, &name) {
                results.push(name);
            }
        }
    }
    results.sort();
    results
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_match_inner(&p, &n)
}

fn glob_match_inner(p: &[char], n: &[char]) -> bool {
    if p.is_empty() { return n.is_empty(); }
    if p[0] == '*' {
        for i in 0..=n.len() {
            if glob_match_inner(&p[1..], &n[i..]) { return true; }
        }
        false
    } else if !n.is_empty() && p[0] == n[0] {
        glob_match_inner(&p[1..], &n[1..])
    } else {
        false
    }
}

fn rand_f64() -> f64 {
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
