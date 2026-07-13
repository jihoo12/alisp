use crate::token::Token;
use crate::expr::Expr;

pub(crate) struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub(crate) fn new(tokens: Vec<Token>) -> Self {
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

    pub(crate) fn parse_all(&mut self) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        while self.pos < self.tokens.len() {
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }
}

pub fn parse(input: &str) -> Result<Vec<Expr>, String> {
    let tokens = crate::token::tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_all()
}

pub fn parse_first(input: &str) -> Result<Expr, String> {
    let tokens = crate::token::tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_expr()
}

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
