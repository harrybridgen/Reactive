use crate::grammar::Token;

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '0'..='9' => {
                let mut value = c.to_digit(10).unwrap();

                while let Some(number) = chars.peek() {
                    if number.is_ascii_digit() {
                        value = value * 10 + number.to_digit(10).unwrap();
                        chars.next();
                    } else {
                        break;
                    }
                }

                tokens.push(Token::Number(value.try_into().unwrap()));
            }

            'a'..='z' | 'A'..='Z' => {
                let mut s = String::new();
                s.push(c);

                while let Some(ch) = chars.peek() {
                    if ch.is_alphanumeric() {
                        s.push(*ch);
                        chars.next();
                    } else {
                        break;
                    }
                }

                let token = match s.as_str() {
                    "print"   => Token::Print,
                    "println" => Token::Println,
                    "if"      => Token::If,
                    "else"    => Token::Else,
                    "loop"    => Token::Loop,
                    "break"   => Token::Break,
                    "func" =>   Token::Func,
                    "return" => Token::Return,
                    "struct" => Token::Struct,
                    _         => Token::Ident(s),
                };

                tokens.push(token);
            }
            '.' => tokens.push(Token::Dot),
            ',' => tokens.push(Token::Comma),
            ':' => {
                if let Some(':') = chars.peek() {
                    chars.next();
                    if let Some('=') = chars.next() {
                        tokens.push(Token::ReactiveAssign);
                    } else {
                        panic!("Did not find matching '=' for reactive eval '::'")
                    }
                } else if let Some('=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::ImmutableAssign);
                } else {
                    panic!("Did not find matching '=' for immutable ':'")
                }
            }
            '|' => {
                if let Some('|') = chars.peek() {
                    chars.next();
                    tokens.push(Token::Or);
                } else {
                    panic!("Did not find matching '|' for Or '||'")
                }
            }
            '!' => {
                if let Some('=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::NotEqual);
                } else {
                    chars.next();
                    tokens.push(Token::Not);
                }
            }
            '&' => {
                if let Some('&') = chars.peek() {
                    chars.next();
                    tokens.push(Token::And);
                } else {
                    panic!("Did not find matching '&' for And '&&'")
                }
            }
            '>' => {
                if let Some('=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::GreaterEqual);
                } else {
                    chars.next();
                    tokens.push(Token::Greater);
                }
            }
            '<' => {
                if let Some('=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::LessEqual);
                } else {
                    chars.next();
                    tokens.push(Token::Less);
                }
            }
            '{' => tokens.push(Token::LBrace),
            '}' => tokens.push(Token::RBrace),
            '[' => tokens.push(Token::LSquare),
            ']' => tokens.push(Token::RSquare),
            ';' => tokens.push(Token::Semicolon),
            '=' => {
                if let Some('=') = chars.next() {
                    tokens.push(Token::Equal);
                } else {
                    tokens.push(Token::Assign);
                }
            }
            '+' => tokens.push(Token::Add),
            '*' => tokens.push(Token::Mul),
            '/' => tokens.push(Token::Div),
            '-' => tokens.push(Token::Sub),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            '#' => {
                for char in chars.by_ref() {
                    if char == '#' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            _ => panic!("[tokenizer] invalid char: {c}"),
        }
    }

    tokens
}
