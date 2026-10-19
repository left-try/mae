use std::fmt;

use mae_domain::DomainError;

/// Lightweight recursive JSON value for pure `std` parsing and serialization.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn parse(input: &str) -> Result<Self, DomainError> {
        let mut chars = input.chars().peekable();
        skip_ws(&mut chars);
        let val = parse_value(&mut chars)?;
        skip_ws(&mut chars);
        Ok(val)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(n) => Some(*n as u64),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(|v| v.as_u64())
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }

    pub fn get_array(&self, key: &str) -> Option<&[JsonValue]> {
        match self.get(key) {
            Some(Self::Array(arr)) => Some(arr.as_slice()),
            _ => None,
        }
    }

    pub fn get_object(&self, key: &str) -> Option<&[(String, JsonValue)]> {
        match self.get(key) {
            Some(Self::Object(entries)) => Some(entries.as_slice()),
            _ => None,
        }
    }

    pub fn to_pretty_json(&self, indent_level: usize) -> String {
        let indent = "  ".repeat(indent_level);
        let next_indent = "  ".repeat(indent_level + 1);

        match self {
            Self::Null => "null".to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Number(n) => {
                if n.fract() == 0.0 && !n.is_nan() && !n.is_infinite() {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Self::String(s) => escape_json_string(s),
            Self::Array(arr) => {
                if arr.is_empty() {
                    "[]".to_string()
                } else {
                    let items: Vec<String> = arr
                        .iter()
                        .map(|item| {
                            format!("{}{}", next_indent, item.to_pretty_json(indent_level + 1))
                        })
                        .collect();
                    format!("[\n{}\n{}]", items.join(",\n"), indent)
                }
            }
            Self::Object(entries) => {
                if entries.is_empty() {
                    "{}".to_string()
                } else {
                    let items: Vec<String> = entries
                        .iter()
                        .map(|(k, v)| {
                            format!(
                                "{}{}: {}",
                                next_indent,
                                escape_json_string(k),
                                v.to_pretty_json(indent_level + 1)
                            )
                        })
                        .collect();
                    format!("{{\n{}\n{}}}", items.join(",\n"), indent)
                }
            }
        }
    }
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_pretty_json(0))
    }
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn skip_ws<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn parse_value<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, DomainError> {
    skip_ws(chars);
    let next_char = chars
        .peek()
        .copied()
        .ok_or_else(|| DomainError::PortError("Unexpected EOF in JSON".into()))?;

    match next_char {
        '{' => parse_object(chars),
        '[' => parse_array(chars),
        '"' => parse_string(chars).map(JsonValue::String),
        't' => parse_literal(chars, "true", JsonValue::Bool(true)),
        'f' => parse_literal(chars, "false", JsonValue::Bool(false)),
        'n' => parse_literal(chars, "null", JsonValue::Null),
        '-' | '0'..='9' => parse_number(chars),
        other => Err(DomainError::PortError(format!(
            "Unexpected character '{}' in JSON",
            other
        ))),
    }
}

fn parse_literal<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
    expected: &str,
    val: JsonValue,
) -> Result<JsonValue, DomainError> {
    for expected_char in expected.chars() {
        match chars.next() {
            Some(c) if c == expected_char => {}
            Some(other) => {
                return Err(DomainError::PortError(format!(
                    "Expected literal '{}', got '{}'",
                    expected, other
                )))
            }
            None => {
                return Err(DomainError::PortError(format!(
                    "Unexpected EOF parsing literal '{}'",
                    expected
                )))
            }
        }
    }
    Ok(val)
}

fn parse_string<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<String, DomainError> {
    if chars.next() != Some('"') {
        return Err(DomainError::PortError("Expected '\"' for string".into()));
    }
    let mut s = String::new();
    while let Some(c) = chars.next() {
        if c == '"' {
            return Ok(s);
        } else if c == '\\' {
            match chars.next() {
                Some('"') => s.push('"'),
                Some('\\') => s.push('\\'),
                Some('n') => s.push('\n'),
                Some('r') => s.push('\r'),
                Some('t') => s.push('\t'),
                Some(other) => s.push(other),
                None => return Err(DomainError::PortError("Unterminated escape in JSON".into())),
            }
        } else {
            s.push(c);
        }
    }
    Err(DomainError::PortError("Unterminated string in JSON".into()))
}

fn parse_number<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, DomainError> {
    let mut num_str = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }
    let num = num_str
        .parse::<f64>()
        .map_err(|e| DomainError::PortError(format!("Invalid number '{}': {}", num_str, e)))?;
    Ok(JsonValue::Number(num))
}

fn parse_object<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, DomainError> {
    chars.next(); // consume '{'
    skip_ws(chars);

    let mut entries = Vec::new();
    if chars.peek() == Some(&'}') {
        chars.next();
        return Ok(JsonValue::Object(entries));
    }

    loop {
        skip_ws(chars);
        let key = parse_string(chars)?;
        skip_ws(chars);
        if chars.next() != Some(':') {
            return Err(DomainError::PortError("Expected ':' in JSON object".into()));
        }
        skip_ws(chars);
        let val = parse_value(chars)?;
        entries.push((key, val));
        skip_ws(chars);

        match chars.peek() {
            Some(',') => {
                chars.next();
                skip_ws(chars);
                if chars.peek() == Some(&'}') {
                    // Trailing comma allowed
                    chars.next();
                    break;
                }
            }
            Some('}') => {
                chars.next();
                break;
            }
            other => {
                return Err(DomainError::PortError(format!(
                    "Expected ',' or '}}' in JSON object, got {:?}",
                    other
                )))
            }
        }
    }

    Ok(JsonValue::Object(entries))
}

fn parse_array<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, DomainError> {
    chars.next(); // consume '['
    skip_ws(chars);

    let mut elements = Vec::new();
    if chars.peek() == Some(&']') {
        chars.next();
        return Ok(JsonValue::Array(elements));
    }

    loop {
        skip_ws(chars);
        let val = parse_value(chars)?;
        elements.push(val);
        skip_ws(chars);

        match chars.peek() {
            Some(',') => {
                chars.next();
                skip_ws(chars);
                if chars.peek() == Some(&']') {
                    // Trailing comma allowed
                    chars.next();
                    break;
                }
            }
            Some(']') => {
                chars.next();
                break;
            }
            other => {
                return Err(DomainError::PortError(format!(
                    "Expected ',' or ']' in JSON array, got {:?}",
                    other
                )))
            }
        }
    }

    Ok(JsonValue::Array(elements))
}
