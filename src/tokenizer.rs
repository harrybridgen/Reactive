use crate::grammar::Token;
use std::iter::Peekable;
use std::str::Chars;

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '0'..='9' => tokens.push(read_number(c, &mut chars)),
            'a'..='z' | 'A'..='Z' => tokens.push(read_ident(c, &mut chars)),
            '.' => tokens.push(Token::Dot),
            ',' => tokens.push(Token::Comma),
            '?' => tokens.push(Token::Question),
            '%' => tokens.push(Token::Modulo),
            '{' => tokens.push(Token::LBrace),
            '}' => tokens.push(Token::RBrace),
            '[' => tokens.push(Token::LSquare),
            ']' => tokens.push(Token::RSquare),
            ';' => tokens.push(Token::Semicolon),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            '+' => tokens.push(Token::Add),
            '*' => tokens.push(Token::Mul),
            '/' => tokens.push(Token::Div),
            '-' => tokens.push(Token::Sub),

            ':' => match chars.peek() {
                Some(':') => {
                    chars.next();
                    match chars.next() {
                        Some('=') => tokens.push(Token::ReactiveAssign),
                        _ => panic!("Expected '=' after '::'"),
                    }
                }
                Some('=') => {
                    chars.next();
                    tokens.push(Token::ImmutableAssign);
                }
                _ => tokens.push(Token::Colon),
            },

            '=' => match chars.peek() {
                Some('=') => {
                    chars.next();
                    tokens.push(Token::Equal);
                }
                _ => tokens.push(Token::Assign),
            },

            '|' => match chars.peek() {
                Some('|') => {
                    chars.next();
                    tokens.push(Token::Or);
                }
                _ => panic!("Expected '||'"),
            },

            '&' => match chars.peek() {
                Some('&') => {
                    chars.next();
                    tokens.push(Token::And);
                }
                _ => panic!("Expected '&&'"),
            },

            '!' => match chars.peek() {
                Some('=') => {
                    chars.next();
                    tokens.push(Token::NotEqual);
                }
                _ => tokens.push(Token::Not),
            },

            '>' => match chars.peek() {
                Some('=') => {
                    chars.next();
                    tokens.push(Token::GreaterEqual);
                }
                _ => tokens.push(Token::Greater),
            },

            '<' => match chars.peek() {
                Some('=') => {
                    chars.next();
                    tokens.push(Token::LessEqual);
                }
                _ => tokens.push(Token::Less),
            },

            '#' => skip_comment(&mut chars),

            '\'' => tokens.push(read_char(&mut chars)),
            '"' => tokens.push(read_string(&mut chars)),

            c if c.is_whitespace() => {}
            _ => panic!("[tokenizer] invalid char: {c}"),
        }
    }

    tokens
}

fn read_number(first: char, chars: &mut Peekable<Chars>) -> Token {
    let mut value = first.to_digit(10).unwrap();
    while let Some(c) = chars.peek().copied() {
        if c.is_ascii_digit() {
            chars.next();
            value = value * 10 + c.to_digit(10).unwrap();
        } else {
            break;
        }
    }
    Token::Number(value as i32)
}

fn read_ident(first: char, chars: &mut Peekable<Chars>) -> Token {
    let mut s = String::new();
    s.push(first);

    while let Some(c) = chars.peek().copied() {
        if c.is_alphanumeric() || c == '_' {
            chars.next();
            s.push(c);
        } else {
            break;
        }
    }

    match s.as_str() {
        "print" => Token::Print,
        "println" => Token::Println,
        "if" => Token::If,
        "else" => Token::Else,
        "loop" => Token::Loop,
        "break" => Token::Break,
        "func" => Token::Func,
        "return" => Token::Return,
        "continue" => Token::Continue,
        "struct" => Token::Struct,
        "import" => Token::Import,
        _ => Token::Ident(s),
    }
}

fn read_char(chars: &mut Peekable<Chars>) -> Token {
    let ch = match chars.next() {
        Some('\\') => read_escape(chars),
        Some(c) => c,
        None => panic!("Unterminated char literal"),
    };

    match chars.next() {
        Some('\'') => Token::Char(ch as u32),
        _ => panic!("Unterminated char literal"),
    }
}

fn read_string(chars: &mut Peekable<Chars>) -> Token {
    let mut s = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => s.push(read_escape(chars)),
            c => s.push(c),
        }
    }
    Token::StringLiteral(s)
}

fn read_escape(chars: &mut Peekable<Chars>) -> char {
    match chars.next() {
        Some('n') => '\n',
        Some('t') => '\t',
        Some('r') => '\r',
        Some('"') => '"',
        Some('\'') => '\'',
        Some('\\') => '\\',
        Some(c @ '0'..='7') => {
            let mut value = (c as u32) - ('0' as u32);
            for _ in 0..2 {
                if let Some(d @ '0'..='7') = chars.peek().copied() {
                    chars.next();
                    value = value * 8 + (d as u32 - '0' as u32);
                } else {
                    break;
                }
            }
            char::from_u32(value).expect("Invalid octal escape")
        }
        Some(c) => panic!("Invalid escape sequence: \\{c}"),
        None => panic!("Unterminated escape sequence"),
    }
}

fn skip_comment(chars: &mut Peekable<Chars>) {
    while let Some(c) = chars.next() {
        if c == '#' {
            break;
        }
    }
}
