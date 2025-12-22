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
                    if ch.is_alphanumeric() || *ch == '_' {
                        s.push(*ch);
                        chars.next();
                    } else {
                        break;
                    }
                }

                let token = match s.as_str() {
                    "print" => Token::Print,
                    "println" => Token::Println,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "loop" => Token::Loop,
                    "break" => Token::Break,
                    "func" => Token::Func,
                    "return" => Token::Return,
                    "struct" => Token::Struct,
                    "import" => Token::Import,
                    _ => Token::Ident(s),
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
                        panic!("Expected '=' after '::'");
                    }
                } else if let Some('=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::ImmutableAssign);
                } else {
                    tokens.push(Token::Colon);
                }
            }
            '?' => tokens.push(Token::Question),
            '%' => {
                tokens.push(Token::Modulo);
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
            '\'' => {
                let ch = match chars.next() {
                    Some('\\') => match chars.peek() {
                        Some('n') => {
                            chars.next();
                            '\n'
                        }
                        Some('t') => {
                            chars.next();
                            '\t'
                        }
                        Some('r') => {
                            chars.next();
                            '\r'
                        }
                        Some('\'') => {
                            chars.next();
                            '\''
                        }
                        Some('\\') => {
                            chars.next();
                            '\\'
                        }

                        Some('0'..='7') => {
                            let mut value: u32 = 0;
                            for _ in 0..3 {
                                match chars.peek() {
                                    Some('0'..='7') => {
                                        value =
                                            value * 8 + (chars.next().unwrap() as u32 - '0' as u32);
                                    }
                                    _ => break,
                                }
                            }
                            char::from_u32(value).expect("Invalid octal escape in char literal")
                        }

                        Some(c) => panic!("Invalid escape sequence: \\{c}"),
                        None => panic!("Unterminated escape sequence"),
                    },

                    Some(c) => c,
                    None => panic!("Unterminated char literal"),
                };

                match chars.next() {
                    Some('\'') => tokens.push(Token::Char(ch as u32)),
                    _ => panic!("Unterminated char literal"),
                }
            }

            '"' => {
                let mut s = String::new();

                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.peek() {
                            Some('n') => {
                                chars.next();
                                s.push('\n');
                            }
                            Some('t') => {
                                chars.next();
                                s.push('\t');
                            }
                            Some('r') => {
                                chars.next();
                                s.push('\r');
                            }
                            Some('"') => {
                                chars.next();
                                s.push('"');
                            }
                            Some('\\') => {
                                chars.next();
                                s.push('\\');
                            }

                            Some('0'..='7') => {
                                let mut value = 0;
                                for _ in 0..3 {
                                    if let Some('0'..='7') = chars.peek() {
                                        value =
                                            value * 8 + (chars.next().unwrap() as u8 - b'0') as u32;
                                    } else {
                                        break;
                                    }
                                }
                                s.push(char::from_u32(value).expect("Invalid octal escape"));
                            }

                            Some(c) => panic!("Invalid escape \\{c}"),
                            None => panic!("Unterminated escape"),
                        },

                        c => s.push(c),
                    }
                }

                tokens.push(Token::StringLiteral(s));
            }

            c if c.is_whitespace() => {}
            _ => panic!("[tokenizer] invalid char: {c}"),
        }
    }

    tokens
}
