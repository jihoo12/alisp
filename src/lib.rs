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

mod token;
mod expr;
mod parser;
mod json;
mod regex;
mod evaluator;
mod builtins;

pub use token::{Token, tokenize};
pub use expr::{Expr, is_truthy, expr_to_string, expr_eq, expr_to_num};
pub use expr::{num, int, string, sym, list, nil, bool_val};
pub use parser::{parse, parse_first, count_parens};
pub use json::{json_parse_str, json_stringify};
pub use evaluator::Evaluator;

pub const VERSION: &str = "0.1.0";
