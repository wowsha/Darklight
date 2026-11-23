//! A simplified HTML tokenizer.
//!
//! Handles doctype, comments, start/end tags with attributes, text with a small
//! set of character references, and raw-text elements (`script`, `style`).

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Doctype(String),
    StartTag {
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    EndTag(String),
    Text(String),
    Comment(String),
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut t = Tokenizer {
        chars: input.chars().collect(),
        pos: 0,
        tokens: Vec::new(),
    };
    t.run();
    t.tokens
}

struct Tokenizer {
    chars: Vec<char>,
    pos: usize,
    tokens: Vec<Token>,
}

impl Tokenizer {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn starts_with_ci(&self, s: &str) -> bool {
        s.chars()
            .enumerate()
            .all(|(i, c)| self.peek_at(i).is_some_and(|x| x.eq_ignore_ascii_case(&c)))
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn flush_text(&mut self, text: &mut String) {
        if !text.is_empty() {
            let decoded = decode_entities(text);
            self.tokens.push(Token::Text(decoded));
            text.clear();
        }
    }

    fn run(&mut self) {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c == '<' {
                let start = self.pos;
                if let Some(tok) = self.try_markup() {
                    self.flush_text(&mut text);
                    let raw_text_tag = match &tok {
                        Token::StartTag {
                            name, self_closing, ..
                        } if !self_closing && (name == "script" || name == "style") => {
                            Some(name.clone())
                        }
                        _ => None,
                    };
                    self.tokens.push(tok);
                    if let Some(name) = raw_text_tag {
                        self.read_raw_text(&name);
                    }
                } else {
                    self.pos = start + 1;
                    text.push('<');
                }
            } else {
                text.push(c);
                self.pos += 1;
            }
        }
        self.flush_text(&mut text);
    }

    /// Reads everything up to the matching `</name` as one raw text token.
    fn read_raw_text(&mut self, name: &str) {
        let closer = format!("</{name}");
        let mut raw = String::new();
        while self.peek().is_some() && !self.starts_with_ci(&closer) {
            raw.push(self.chars[self.pos]);
            self.pos += 1;
        }
        if !raw.is_empty() {
            self.tokens.push(Token::Text(raw));
        }
    }

    /// Attempts to read markup starting at `<`. Returns `None` if this `<` is
    /// just text; the caller then resets the position.
    fn try_markup(&mut self) -> Option<Token> {
        match self.peek_at(1)? {
            '!' => self.read_bang(),
            '/' => self.read_end_tag(),
            c if c.is_ascii_alphabetic() => self.read_start_tag(),
            _ => None,
        }
    }

    fn read_bang(&mut self) -> Option<Token> {
        if self.starts_with_ci("<!--") {
            self.pos += 4;
            let mut body = String::new();
            while self.peek().is_some() {
                if self.starts_with_ci("-->") {
                    self.pos += 3;
                    return Some(Token::Comment(body));
                }
                body.push(self.chars[self.pos]);
                self.pos += 1;
            }
            // Unterminated comment: consume to EOF, as browsers do.
            return Some(Token::Comment(body));
        }
        let is_doctype = self.starts_with_ci("<!doctype");
        self.pos += 2;
        let mut body = String::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == '>' {
                break;
            }
            body.push(c);
        }
        if is_doctype {
            let name = body["doctype".len().min(body.len())..].trim().to_string();
            Some(Token::Doctype(name))
        } else {
            Some(Token::Comment(body))
        }
    }

    fn read_end_tag(&mut self) -> Option<Token> {
        if !self.peek_at(2)?.is_ascii_alphabetic() {
            return None;
        }
        self.pos += 2;
        let name = self.read_name();
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == '>' {
                break;
            }
        }
        Some(Token::EndTag(name))
    }

    fn read_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() || c == '/' || c == '>' {
                break;
            }
            name.push(c.to_ascii_lowercase());
            self.pos += 1;
        }
        name
    }

    fn read_start_tag(&mut self) -> Option<Token> {
        self.pos += 1;
        let name = self.read_name();
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        loop {
            self.skip_ws();
            match self.peek() {
                None => return None, // unterminated tag: treat `<` as text
                Some('>') => {
                    self.pos += 1;
                    break;
                }
                Some('/') => {
                    self.pos += 1;
                    if self.peek() == Some('>') {
                        self_closing = true;
                        self.pos += 1;
                        break;
                    }
                }
                Some(_) => {
                    let (n, v) = self.read_attribute();
                    // The first occurrence of a duplicate attribute wins.
                    if !attrs.iter().any(|(existing, _)| *existing == n) {
                        attrs.push((n, v));
                    }
                }
            }
        }
        Some(Token::StartTag {
            name,
            attrs,
            self_closing,
        })
    }

    fn read_attribute(&mut self) -> (String, String) {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() || c == '=' || c == '>' || c == '/' {
                break;
            }
            name.push(c.to_ascii_lowercase());
            self.pos += 1;
        }
        if name.is_empty() {
            // Stray character such as a lone `=`; skip it so the loop advances.
            self.pos += 1;
            return (String::new(), String::new());
        }
        self.skip_ws();
        let mut value = String::new();
        if self.peek() == Some('=') {
            self.pos += 1;
            self.skip_ws();
            match self.peek() {
                Some(q @ ('"' | '\'')) => {
                    self.pos += 1;
                    while let Some(c) = self.peek() {
                        self.pos += 1;
                        if c == q {
                            break;
                        }
                        value.push(c);
                    }
                }
                _ => {
                    while let Some(c) = self.peek() {
                        if c.is_ascii_whitespace() || c == '>' {
                            break;
                        }
                        value.push(c);
                        self.pos += 1;
                    }
                }
            }
        }
        (name, decode_entities(&value))
    }
}

/// Decodes a small subset of character references: the common named ones plus
/// decimal and hex numeric references. Unknown references are left untouched.
pub fn decode_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == ';') {
                let name: String = chars[i + 1..i + 1 + end].iter().collect();
                if let Some(decoded) = decode_reference(&name) {
                    out.push(decoded);
                    i += end + 2;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn decode_reference(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => {
            let digits = name.strip_prefix('#')?;
            let code = if let Some(hex) = digits.strip_prefix(['x', 'X']) {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                digits.parse::<u32>().ok()?
            };
            char::from_u32(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_tags_attributes_and_text() {
        let toks = tokenize(r#"<a href="x.html" class=big disabled>hi</a>"#);
        assert_eq!(
            toks,
            vec![
                Token::StartTag {
                    name: "a".into(),
                    attrs: vec![
                        ("href".into(), "x.html".into()),
                        ("class".into(), "big".into()),
                        ("disabled".into(), "".into()),
                    ],
                    self_closing: false,
                },
                Token::Text("hi".into()),
                Token::EndTag("a".into()),
            ]
        );
    }

    #[test]
    fn lowercases_names_and_keeps_first_duplicate_attribute() {
        let toks = tokenize(r#"<DIV ID="a" id="b">"#);
        assert_eq!(
            toks,
            vec![Token::StartTag {
                name: "div".into(),
                attrs: vec![("id".into(), "a".into())],
                self_closing: false,
            }]
        );
    }

    #[test]
    fn comments_and_doctype() {
        let toks = tokenize("<!DOCTYPE html><!-- hello --><p>");
        assert_eq!(toks[0], Token::Doctype("html".into()));
        assert_eq!(toks[1], Token::Comment(" hello ".into()));
    }

    #[test]
    fn decodes_entities() {
        assert_eq!(
            decode_entities("a &amp; b &lt;c&gt; &#65;&#x42; &bogus;"),
            "a & b <c> AB &bogus;"
        );
    }

    #[test]
    fn script_content_is_raw_text() {
        let toks = tokenize("<script>if (a < b && c) {}</script>");
        assert_eq!(toks[1], Token::Text("if (a < b && c) {}".into()));
        assert_eq!(toks[2], Token::EndTag("script".into()));
    }

    #[test]
    fn lone_less_than_is_text() {
        assert_eq!(tokenize("1 < 2"), vec![Token::Text("1 < 2".into())]);
    }

    #[test]
    fn unterminated_tag_is_text() {
        assert_eq!(
            tokenize("<div class=\"x"),
            vec![Token::Text("<div class=\"x".into())]
        );
    }

    #[test]
    fn handles_non_ascii_text() {
        assert_eq!(tokenize("héllo ✨"), vec![Token::Text("héllo ✨".into())]);
    }
}
