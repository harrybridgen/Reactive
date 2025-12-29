use crate::grammar::{CastType, CompiledStructFieldInit, Instruction, ReactiveExpr};
use std::fs;

const MAGIC: &str = "RXB1";

pub fn serialize_instructions(code: &[Instruction]) -> String {
    let mut lines = Vec::new();
    lines.push(MAGIC.to_string());
    for instr in code {
        write_instruction(instr, &mut lines);
    }
    lines.join("\n")
}

pub fn deserialize_instructions(input: &str) -> Result<Vec<Instruction>, String> {
    let mut lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return Err("bytecode is empty".to_string());
    }
    let header = lines.remove(0);
    if header.trim() != MAGIC {
        return Err(format!("invalid bytecode header: expected {MAGIC}"));
    }

    let mut parser = Parser::new(lines);
    let mut instructions = Vec::new();
    while !parser.is_done() {
        instructions.push(parser.parse_instruction()?);
    }
    Ok(instructions)
}

pub fn write_instructions_to_file(path: &str, code: &[Instruction]) -> std::io::Result<()> {
    fs::write(path, serialize_instructions(code))
}

pub fn read_instructions_from_file(path: &str) -> Result<Vec<Instruction>, String> {
    let input = fs::read_to_string(path)
        .map_err(|e| format!("failed to read bytecode `{}`: {}", path, e))?;
    deserialize_instructions(&input)
}

fn write_instruction(instr: &Instruction, lines: &mut Vec<String>) {
    match instr {
        Instruction::Push(n) => lines.push(format!("Push {n}")),
        Instruction::PushChar(c) => lines.push(format!("PushChar {c}")),
        Instruction::Load(name) => lines.push(format!("Load {}", quote_string(name))),

        Instruction::Store(name) => lines.push(format!("Store {}", quote_string(name))),
        Instruction::StoreImmutable(name) => {
            lines.push(format!("StoreImmutable {}", quote_string(name)))
        }
        Instruction::StoreReactive(name, expr) => {
            write_reactive_header("StoreReactive", Some(name), expr, lines);
        }

        Instruction::Add => lines.push("Add".to_string()),
        Instruction::Sub => lines.push("Sub".to_string()),
        Instruction::Mul => lines.push("Mul".to_string()),
        Instruction::Div => lines.push("Div".to_string()),
        Instruction::Modulo => lines.push("Modulo".to_string()),

        Instruction::Greater => lines.push("Greater".to_string()),
        Instruction::Less => lines.push("Less".to_string()),
        Instruction::GreaterEqual => lines.push("GreaterEqual".to_string()),
        Instruction::LessEqual => lines.push("LessEqual".to_string()),
        Instruction::Equal => lines.push("Equal".to_string()),
        Instruction::NotEqual => lines.push("NotEqual".to_string()),
        Instruction::And => lines.push("And".to_string()),
        Instruction::Or => lines.push("Or".to_string()),

        Instruction::Label(name) => lines.push(format!("Label {}", quote_string(name))),
        Instruction::Jump(name) => lines.push(format!("Jump {}", quote_string(name))),
        Instruction::JumpIfZero(name) => lines.push(format!("JumpIfZero {}", quote_string(name))),
        Instruction::Return => lines.push("Return".to_string()),

        Instruction::ArrayNew => lines.push("ArrayNew".to_string()),
        Instruction::ArrayGet => lines.push("ArrayGet".to_string()),
        Instruction::ArrayLValue => lines.push("ArrayLValue".to_string()),
        Instruction::StoreIndex(name) => lines.push(format!("StoreIndex {}", quote_string(name))),
        Instruction::StoreIndexReactive(name, expr) => {
            write_reactive_header("StoreIndexReactive", Some(name), expr, lines);
        }

        Instruction::StoreStruct(name, fields) => {
            lines.push(format!(
                "StoreStruct {} {}",
                quote_string(name),
                fields.len()
            ));
            for (field_name, init) in fields {
                write_struct_field(field_name, init.as_ref(), lines);
            }
        }
        Instruction::NewStruct(name) => lines.push(format!("NewStruct {}", quote_string(name))),
        Instruction::FieldGet(name) => lines.push(format!("FieldGet {}", quote_string(name))),
        Instruction::FieldSet(name) => lines.push(format!("FieldSet {}", quote_string(name))),
        Instruction::FieldSetReactive(name, expr) => {
            write_reactive_header("FieldSetReactive", Some(name), expr, lines);
        }
        Instruction::FieldLValue(name) => lines.push(format!("FieldLValue {}", quote_string(name))),

        Instruction::StoreThrough => lines.push("StoreThrough".to_string()),
        Instruction::StoreThroughReactive(expr) => {
            write_reactive_header("StoreThroughReactive", None, expr, lines);
        }
        Instruction::StoreThroughImmutable => lines.push("StoreThroughImmutable".to_string()),

        Instruction::StoreFunction(name, params, body) => {
            let mut line = format!("StoreFunction {} {}", quote_string(name), params.len());
            for p in params {
                line.push(' ');
                line.push_str(&quote_string(p));
            }
            line.push(' ');
            line.push_str(&body.len().to_string());
            lines.push(line);
            for instr in body {
                write_instruction(instr, lines);
            }
        }
        Instruction::Call(name, argc) => {
            lines.push(format!("Call {} {}", quote_string(name), argc))
        }

        Instruction::PushImmutableContext => lines.push("PushImmutableContext".to_string()),
        Instruction::PopImmutableContext => lines.push("PopImmutableContext".to_string()),
        Instruction::ClearImmutableContext => lines.push("ClearImmutableContext".to_string()),

        Instruction::Print => lines.push("Print".to_string()),
        Instruction::Println => lines.push("Println".to_string()),
        Instruction::Assert => lines.push("Assert".to_string()),
        Instruction::Error(message) => lines.push(format!("Error {}", quote_string(message))),

        Instruction::Import(path) => {
            let mut line = format!("Import {}", path.len());
            for segment in path {
                line.push(' ');
                line.push_str(&quote_string(segment));
            }
            lines.push(line);
        }

        Instruction::Cast(target) => {
            let tag = match target {
                CastType::Int => "Int",
                CastType::Char => "Char",
            };
            lines.push(format!("Cast {tag}"));
        }
    }
}

fn write_struct_field(name: &str, init: Option<&CompiledStructFieldInit>, lines: &mut Vec<String>) {
    match init {
        None => lines.push(format!("Field {} None", quote_string(name))),
        Some(CompiledStructFieldInit::Mutable(code)) => {
            lines.push(format!(
                "Field {} Mutable {}",
                quote_string(name),
                code.len()
            ));
            for instr in code {
                write_instruction(instr, lines);
            }
        }
        Some(CompiledStructFieldInit::Immutable(code)) => {
            lines.push(format!(
                "Field {} Immutable {}",
                quote_string(name),
                code.len()
            ));
            for instr in code {
                write_instruction(instr, lines);
            }
        }
        Some(CompiledStructFieldInit::Reactive(expr)) => {
            write_reactive_field(name, expr, lines);
        }
    }
}

fn write_reactive_field(name: &str, expr: &ReactiveExpr, lines: &mut Vec<String>) {
    let mut line = format!("Field {} Reactive {}", quote_string(name), expr.captures.len());
    for cap in &expr.captures {
        line.push(' ');
        line.push_str(&quote_string(cap));
    }
    line.push(' ');
    line.push_str(&expr.code.len().to_string());
    lines.push(line);
    for instr in &expr.code {
        write_instruction(instr, lines);
    }
}

fn write_reactive_header(
    keyword: &str,
    name: Option<&str>,
    expr: &ReactiveExpr,
    lines: &mut Vec<String>,
) {
    let mut line = String::new();
    line.push_str(keyword);
    if let Some(name) = name {
        line.push(' ');
        line.push_str(&quote_string(name));
    }
    line.push(' ');
    line.push_str(&expr.captures.len().to_string());
    for cap in &expr.captures {
        line.push(' ');
        line.push_str(&quote_string(cap));
    }
    line.push(' ');
    line.push_str(&expr.code.len().to_string());
    lines.push(line);
    for instr in &expr.code {
        write_instruction(instr, lines);
    }
}

fn quote_string(s: &str) -> String {
    let mut out = String::new();
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let code = c as u32;
                out.push_str(&format!("\\u{{{:x}}}", code));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    index: usize,
    last_line: usize,
}

impl<'a> Parser<'a> {
    fn new(lines: Vec<&'a str>) -> Self {
        Self {
            lines,
            index: 0,
            last_line: 0,
        }
    }

    fn is_done(&self) -> bool {
        self.index >= self.lines.len()
    }

    fn parse_instruction(&mut self) -> Result<Instruction, String> {
        let line = self.next_line()?;
        let tokens = tokenize_line(line).map_err(|e| self.error(&e))?;
        if tokens.is_empty() {
            return Err(self.error("empty instruction line"));
        }
        let op = tokens[0].as_str();
        match op {
            "Push" => parse_arity(&tokens, 2, op, self)
                .and_then(|_| parse_i32(&tokens[1]).map(Instruction::Push)),
            "PushChar" => parse_arity(&tokens, 2, op, self)
                .and_then(|_| parse_u32(&tokens[1]).map(Instruction::PushChar)),
            "Load" => {
                parse_arity(&tokens, 2, op, self).map(|_| Instruction::Load(tokens[1].clone()))
            }

            "Store" => {
                parse_arity(&tokens, 2, op, self).map(|_| Instruction::Store(tokens[1].clone()))
            }
            "StoreImmutable" => parse_arity(&tokens, 2, op, self)
                .map(|_| Instruction::StoreImmutable(tokens[1].clone())),
            "StoreReactive" => self.parse_reactive_named(tokens, Instruction::StoreReactive),

            "Add" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Add),
            "Sub" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Sub),
            "Mul" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Mul),
            "Div" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Div),
            "Modulo" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Modulo),

            "Greater" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Greater),
            "Less" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Less),
            "GreaterEqual" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::GreaterEqual),
            "LessEqual" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::LessEqual),
            "Equal" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Equal),
            "NotEqual" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::NotEqual),
            "And" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::And),
            "Or" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Or),

            "Label" => {
                parse_arity(&tokens, 2, op, self).map(|_| Instruction::Label(tokens[1].clone()))
            }
            "Jump" => {
                parse_arity(&tokens, 2, op, self).map(|_| Instruction::Jump(tokens[1].clone()))
            }
            "JumpIfZero" => parse_arity(&tokens, 2, op, self)
                .map(|_| Instruction::JumpIfZero(tokens[1].clone())),
            "Return" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Return),

            "ArrayNew" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::ArrayNew),
            "ArrayGet" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::ArrayGet),
            "ArrayLValue" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::ArrayLValue),
            "StoreIndex" => parse_arity(&tokens, 2, op, self)
                .map(|_| Instruction::StoreIndex(tokens[1].clone())),
            "StoreIndexReactive" => {
                self.parse_reactive_named(tokens, Instruction::StoreIndexReactive)
            }

            "StoreStruct" => self.parse_struct(tokens),
            "NewStruct" => {
                parse_arity(&tokens, 2, op, self).map(|_| Instruction::NewStruct(tokens[1].clone()))
            }
            "FieldGet" => {
                parse_arity(&tokens, 2, op, self).map(|_| Instruction::FieldGet(tokens[1].clone()))
            }
            "FieldSet" => {
                parse_arity(&tokens, 2, op, self).map(|_| Instruction::FieldSet(tokens[1].clone()))
            }
            "FieldSetReactive" => self.parse_reactive_named(tokens, Instruction::FieldSetReactive),
            "FieldLValue" => parse_arity(&tokens, 2, op, self)
                .map(|_| Instruction::FieldLValue(tokens[1].clone())),

            "StoreThrough" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::StoreThrough),
            "StoreThroughReactive" => self.parse_reactive_unnamed(tokens),
            "StoreThroughImmutable" => {
                parse_arity(&tokens, 1, op, self).map(|_| Instruction::StoreThroughImmutable)
            }

            "StoreFunction" => self.parse_function(tokens),
            "Call" => parse_arity(&tokens, 3, op, self).and_then(|_| {
                parse_usize(&tokens[2]).map(|argc| Instruction::Call(tokens[1].clone(), argc))
            }),

            "PushImmutableContext" => {
                parse_arity(&tokens, 1, op, self).map(|_| Instruction::PushImmutableContext)
            }
            "PopImmutableContext" => {
                parse_arity(&tokens, 1, op, self).map(|_| Instruction::PopImmutableContext)
            }
            "ClearImmutableContext" => {
                parse_arity(&tokens, 1, op, self).map(|_| Instruction::ClearImmutableContext)
            }

            "Print" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Print),
            "Println" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Println),
            "Assert" => parse_arity(&tokens, 1, op, self).map(|_| Instruction::Assert),
            "Error" => {
                parse_arity(&tokens, 2, op, self).map(|_| Instruction::Error(tokens[1].clone()))
            }

            "Import" => self.parse_import(tokens),

            "Cast" => parse_arity(&tokens, 2, op, self).and_then(|_| {
                let target = match tokens[1].as_str() {
                    "Int" => CastType::Int,
                    "Char" => CastType::Char,
                    other => return Err(self.error(&format!("unknown cast type `{}`", other))),
                };
                Ok(Instruction::Cast(target))
            }),
            other => Err(self.error(&format!("unknown instruction `{}`", other))),
        }
    }

    fn parse_import(&mut self, tokens: Vec<String>) -> Result<Instruction, String> {
        if tokens.len() < 2 {
            return Err(self.error("Import expects a count"));
        }
        let count = parse_usize(&tokens[1]).map_err(|e| self.error(&e))?;
        let expected = 2 + count;
        if tokens.len() != expected {
            return Err(self.error(&format!("Import expects {} segment(s)", count)));
        }
        let mut segments = Vec::with_capacity(count);
        for seg in tokens.into_iter().skip(2) {
            segments.push(seg);
        }
        Ok(Instruction::Import(segments))
    }

    fn parse_function(&mut self, tokens: Vec<String>) -> Result<Instruction, String> {
        if tokens.len() < 4 {
            return Err(self.error("StoreFunction expects name, param count, params, code length"));
        }
        let name = tokens[1].clone();
        let param_count = parse_usize(&tokens[2]).map_err(|e| self.error(&e))?;
        let expected = 4 + param_count;
        if tokens.len() != expected {
            return Err(self.error(&format!(
                "StoreFunction expects {} parameter(s)",
                param_count
            )));
        }
        let mut params = Vec::with_capacity(param_count);
        for p in tokens.iter().skip(3).take(param_count) {
            params.push(p.clone());
        }
        let code_len = parse_usize(&tokens[3 + param_count]).map_err(|e| self.error(&e))?;
        let code = self.parse_instructions(code_len)?;
        Ok(Instruction::StoreFunction(name, params, code))
    }

    fn parse_struct(&mut self, tokens: Vec<String>) -> Result<Instruction, String> {
        if tokens.len() != 3 {
            return Err(self.error("StoreStruct expects name and field count"));
        }
        let name = tokens[1].clone();
        let field_count = parse_usize(&tokens[2]).map_err(|e| self.error(&e))?;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            fields.push(self.parse_field()?);
        }
        Ok(Instruction::StoreStruct(name, fields))
    }

    fn parse_field(&mut self) -> Result<(String, Option<CompiledStructFieldInit>), String> {
        let line = self.next_line()?;
        let tokens = tokenize_line(line).map_err(|e| self.error(&e))?;
        if tokens.len() < 3 || tokens[0] != "Field" {
            return Err(self.error("expected Field entry"));
        }
        let name = tokens[1].clone();
        let kind = tokens[2].as_str();
        match kind {
            "None" => {
                if tokens.len() != 3 {
                    return Err(self.error("Field None expects no extra tokens"));
                }
                Ok((name, None))
            }
            "Mutable" => {
                if tokens.len() != 4 {
                    return Err(self.error("Field Mutable expects code length"));
                }
                let code_len = parse_usize(&tokens[3]).map_err(|e| self.error(&e))?;
                let code = self.parse_instructions(code_len)?;
                Ok((name, Some(CompiledStructFieldInit::Mutable(code))))
            }
            "Immutable" => {
                if tokens.len() != 4 {
                    return Err(self.error("Field Immutable expects code length"));
                }
                let code_len = parse_usize(&tokens[3]).map_err(|e| self.error(&e))?;
                let code = self.parse_instructions(code_len)?;
                Ok((name, Some(CompiledStructFieldInit::Immutable(code))))
            }
            "Reactive" => {
                if tokens.len() < 5 {
                    return Err(self.error("Field Reactive expects captures and code length"));
                }
                let cap_count = parse_usize(&tokens[3]).map_err(|e| self.error(&e))?;
                let expected = 5 + cap_count;
                if tokens.len() != expected {
                    return Err(
                        self.error(&format!("Field Reactive expects {} capture(s)", cap_count))
                    );
                }
                let captures = tokens[4..4 + cap_count].to_vec();
                let code_len = parse_usize(&tokens[4 + cap_count]).map_err(|e| self.error(&e))?;
                let code = self.parse_instructions(code_len)?;
                Ok((
                    name,
                    Some(CompiledStructFieldInit::Reactive(ReactiveExpr {
                        code,
                        captures,
                    })),
                ))
            }
            other => Err(self.error(&format!("unknown field init `{}`", other))),
        }
    }

    fn parse_reactive_named(
        &mut self,
        tokens: Vec<String>,
        ctor: fn(String, ReactiveExpr) -> Instruction,
    ) -> Result<Instruction, String> {
        if tokens.len() < 4 {
            return Err(self.error("expected name, capture count, captures, code length"));
        }
        let name = tokens[1].clone();
        let cap_count = parse_usize(&tokens[2]).map_err(|e| self.error(&e))?;
        let expected = 4 + cap_count;
        if tokens.len() != expected {
            return Err(self.error(&format!("expected {} capture(s)", cap_count)));
        }
        let captures = tokens[3..3 + cap_count].to_vec();
        let code_len = parse_usize(&tokens[3 + cap_count]).map_err(|e| self.error(&e))?;
        let code = self.parse_instructions(code_len)?;
        Ok(ctor(name, ReactiveExpr { code, captures }))
    }

    fn parse_reactive_unnamed(&mut self, tokens: Vec<String>) -> Result<Instruction, String> {
        if tokens.len() < 3 {
            return Err(self.error("expected capture count, captures, code length"));
        }
        let cap_count = parse_usize(&tokens[1]).map_err(|e| self.error(&e))?;
        let expected = 3 + cap_count;
        if tokens.len() != expected {
            return Err(self.error(&format!("expected {} capture(s)", cap_count)));
        }
        let captures = tokens[2..2 + cap_count].to_vec();
        let code_len = parse_usize(&tokens[2 + cap_count]).map_err(|e| self.error(&e))?;
        let code = self.parse_instructions(code_len)?;
        Ok(Instruction::StoreThroughReactive(ReactiveExpr {
            code,
            captures,
        }))
    }

    fn parse_instructions(&mut self, count: usize) -> Result<Vec<Instruction>, String> {
        let mut code = Vec::with_capacity(count);
        for _ in 0..count {
            code.push(self.parse_instruction()?);
        }
        Ok(code)
    }

    fn next_line(&mut self) -> Result<&'a str, String> {
        if self.index >= self.lines.len() {
            return Err(self.error("unexpected end of bytecode"));
        }
        let line = self.lines[self.index];
        self.last_line = self.index + 1;
        self.index += 1;
        Ok(line)
    }

    fn error(&self, message: &str) -> String {
        let line = if self.last_line == 0 {
            self.index + 1
        } else {
            self.last_line
        };
        format!("line {}: {}", line, message)
    }
}

fn tokenize_line(line: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch == '"' {
            chars.next();
            let mut out = String::new();
            let mut closed = false;
            while let Some(c) = chars.next() {
                match c {
                    '"' => {
                        closed = true;
                        break;
                    }
                    '\\' => {
                        let esc = chars.next().ok_or("unterminated escape")?;
                        match esc {
                            'n' => out.push('\n'),
                            'r' => out.push('\r'),
                            't' => out.push('\t'),
                            '\\' => out.push('\\'),
                            '"' => out.push('"'),
                            'u' => {
                                if chars.next() != Some('{') {
                                    return Err("invalid unicode escape".to_string());
                                }
                                let mut hex = String::new();
                                let mut closed_brace = false;
                                while let Some(&h) = chars.peek() {
                                    if h == '}' {
                                        chars.next();
                                        closed_brace = true;
                                        break;
                                    }
                                    hex.push(h);
                                    chars.next();
                                }
                                if !closed_brace {
                                    return Err("unterminated unicode escape".to_string());
                                }
                                let value = u32::from_str_radix(&hex, 16)
                                    .map_err(|_| "invalid unicode escape".to_string())?;
                                let decoded =
                                    char::from_u32(value).ok_or("invalid unicode scalar")?;
                                out.push(decoded);
                            }
                            other => {
                                return Err(format!("unknown escape `\\{}`", other));
                            }
                        }
                    }
                    other => out.push(other),
                }
            }
            if !closed {
                return Err("unterminated string".to_string());
            }
            tokens.push(out);
        } else {
            let mut out = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                out.push(c);
                chars.next();
            }
            tokens.push(out);
        }
    }
    if tokens.is_empty() {
        return Err("empty line".to_string());
    }
    Ok(tokens)
}

fn parse_arity(
    tokens: &[String],
    expected: usize,
    op: &str,
    parser: &Parser,
) -> Result<(), String> {
    if tokens.len() != expected {
        return Err(parser.error(&format!("{} expects {} token(s)", op, expected)));
    }
    Ok(())
}

fn parse_i32(s: &str) -> Result<i32, String> {
    s.parse::<i32>().map_err(|_| format!("invalid i32 `{}`", s))
}

fn parse_u32(s: &str) -> Result<u32, String> {
    s.parse::<u32>().map_err(|_| format!("invalid u32 `{}`", s))
}

fn parse_usize(s: &str) -> Result<usize, String> {
    s.parse::<usize>()
        .map_err(|_| format!("invalid usize `{}`", s))
}
