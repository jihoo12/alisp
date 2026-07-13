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
