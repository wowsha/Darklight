//! CSS value types and value parsing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Px,
    Em,
    Percent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// An identifier such as `auto` or `block`. Also the fallback that holds the
    /// raw text of any value this parser does not understand yet.
    Keyword(String),
    Length(f32, Unit),
    Number(f32),
    Color(Color),
    /// A whitespace-separated list such as `margin: 1px 2px`.
    List(Vec<Value>),
}

/// Parses the value part of a declaration (without `!important`).
pub fn parse_value(input: &str) -> Value {
    let input = input.trim();
    // Anything with a function call is kept raw until real tokenization lands.
    if !input.contains('(') {
        let parts: Vec<&str> = input.split_ascii_whitespace().collect();
        if parts.len() > 1 {
            return Value::List(parts.into_iter().map(parse_single).collect());
        }
    }
    parse_single(input)
}

fn parse_single(token: &str) -> Value {
    if let Some(hex) = token.strip_prefix('#') {
        if let Some(c) = parse_hex_color(hex) {
            return Value::Color(c);
        }
    }
    if let Some(c) = named_color(&token.to_ascii_lowercase()) {
        return Value::Color(c);
    }
    if let Some(v) = parse_numeric(token) {
        return v;
    }
    Value::Keyword(token.to_string())
}

fn parse_numeric(token: &str) -> Option<Value> {
    let split = token
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
        .unwrap_or(token.len());
    let (num, unit) = token.split_at(split);
    let n: f32 = num.parse().ok()?;
    match unit.to_ascii_lowercase().as_str() {
        "" => Some(Value::Number(n)),
        "px" => Some(Value::Length(n, Unit::Px)),
        "em" => Some(Value::Length(n, Unit::Em)),
        "%" => Some(Value::Length(n, Unit::Percent)),
        _ => None,
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let nib = |c: char| c.to_digit(16).map(|d| (d * 17) as u8);
    let byte = |s: &str| u8::from_str_radix(s, 16).ok();
    match hex.len() {
        3 => {
            let mut it = hex.chars();
            Some(Color::rgb(
                nib(it.next()?)?,
                nib(it.next()?)?,
                nib(it.next()?)?,
            ))
        }
        6 => Some(Color::rgb(
            byte(&hex[0..2])?,
            byte(&hex[2..4])?,
            byte(&hex[4..6])?,
        )),
        8 => Some(Color {
            r: byte(&hex[0..2])?,
            g: byte(&hex[2..4])?,
            b: byte(&hex[4..6])?,
            a: byte(&hex[6..8])?,
        }),
        _ => None,
    }
}

fn named_color(name: &str) -> Option<Color> {
    Some(match name {
        "black" => Color::BLACK,
        "white" => Color::WHITE,
        "red" => Color::rgb(255, 0, 0),
        "green" => Color::rgb(0, 128, 0),
        "blue" => Color::rgb(0, 0, 255),
        "yellow" => Color::rgb(255, 255, 0),
        "gray" | "grey" => Color::rgb(128, 128, 128),
        "orange" => Color::rgb(255, 165, 0),
        "purple" => Color::rgb(128, 0, 128),
        "transparent" => Color::TRANSPARENT,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colors() {
        assert_eq!(parse_value("#fff"), Value::Color(Color::WHITE));
        assert_eq!(
            parse_value("#102030"),
            Value::Color(Color::rgb(0x10, 0x20, 0x30))
        );
        assert_eq!(parse_value("RED"), Value::Color(Color::rgb(255, 0, 0)));
        assert_eq!(
            parse_value("#ff000080"),
            Value::Color(Color {
                r: 255,
                g: 0,
                b: 0,
                a: 0x80
            })
        );
    }

    #[test]
    fn parses_lengths_and_numbers() {
        assert_eq!(parse_value("12px"), Value::Length(12.0, Unit::Px));
        assert_eq!(parse_value("1.5em"), Value::Length(1.5, Unit::Em));
        assert_eq!(parse_value("50%"), Value::Length(50.0, Unit::Percent));
        assert_eq!(parse_value("0"), Value::Number(0.0));
    }

    #[test]
    fn parses_lists_and_keywords() {
        assert_eq!(
            parse_value("1px auto"),
            Value::List(vec![
                Value::Length(1.0, Unit::Px),
                Value::Keyword("auto".into())
            ])
        );
        assert_eq!(parse_value("block"), Value::Keyword("block".into()));
    }

    #[test]
    fn function_values_stay_raw() {
        assert_eq!(
            parse_value("rgb(1, 2, 3)"),
            Value::Keyword("rgb(1, 2, 3)".into())
        );
    }

    #[test]
    fn bad_hex_falls_back_to_keyword() {
        assert_eq!(parse_value("#zzz"), Value::Keyword("#zzz".into()));
    }
}
