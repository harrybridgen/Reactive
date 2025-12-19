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
                let mut str = String::new();
                str.push(c);

                while let Some(char) = chars.peek() {
                    if char.is_alphanumeric() {
                        str.push(*char);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if str == "print" {
                    tokens.push(Token::Print);
                } else if str == "if" {
                    tokens.push(Token::If);
                } else if str == "else" {
                    tokens.push(Token::Else);
                }else if str == "loop"{
                    tokens.push(Token::Loop);
                }else if str == "break"{
                    tokens.push(Token::Break);
                
                } else {
                    tokens.push(Token::Ident(str));
                }
            }
            ':' => {
                if let Some('=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::LazyAssign);
                } else {
                    panic!("Did not find matching '=' for lazy eval ':'")
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
