//! Stylesheet, rule, selector and declaration parsing.

use crate::value::{Value, parse_value};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
    pub important: bool,
}

/// (ids, classes, types) — compared lexicographically.
pub type Specificity = (u32, u32, u32);

/// A compound of simple selectors, e.g. `div.note#main`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Selector {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
}

impl Selector {
    pub fn specificity(&self) -> Specificity {
        (
            self.id.is_some() as u32,
            self.classes.len() as u32,
            self.tag.is_some() as u32,
        )
    }

    pub fn matches(&self, tag: &str, id: Option<&str>, classes: &[&str]) -> bool {
        if self
            .tag
            .as_deref()
            .is_some_and(|t| !t.eq_ignore_ascii_case(tag))
        {
            return false;
        }
        if self.id.is_some() && self.id.as_deref() != id {
            return false;
        }
        self.classes.iter().all(|c| classes.contains(&c.as_str()))
    }
}

pub fn parse_stylesheet(source: &str) -> Stylesheet {
    let src: Vec<char> = strip_comments(source).chars().collect();
    let mut pos = 0;
    let mut rules = Vec::new();

    while pos < src.len() {
        skip_ws(&src, &mut pos);
        if pos >= src.len() {
            break;
        }
        if src[pos] == '@' {
            skip_at_rule(&src, &mut pos);
            continue;
        }
        let prelude_start = pos;
        while pos < src.len() && src[pos] != '{' {
            pos += 1;
        }
        if pos >= src.len() {
            break; // dangling prelude without a block
        }
        let prelude: String = src[prelude_start..pos].iter().collect();
        pos += 1; // '{'
        let block_start = pos;
        let mut depth = 1;
        while pos < src.len() && depth > 0 {
            match src[pos] {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            pos += 1;
        }
        let block_end = if depth == 0 { pos - 1 } else { pos };
        let block: String = src[block_start..block_end].iter().collect();

        let selectors: Vec<Selector> = prelude.split(',').filter_map(parse_selector).collect();
        if !selectors.is_empty() {
            rules.push(Rule {
                selectors,
                declarations: parse_declarations(&block),
            });
        }
    }
    Stylesheet { rules }
}

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out, // unterminated comment swallows the rest
        }
    }
    out.push_str(rest);
    out
}

fn skip_ws(src: &[char], pos: &mut usize) {
    while *pos < src.len() && src[*pos].is_whitespace() {
        *pos += 1;
    }
}

/// Skips `@name ...;` or `@name ... { ... }` (with nested braces).
fn skip_at_rule(src: &[char], pos: &mut usize) {
    while *pos < src.len() {
        match src[*pos] {
            ';' => {
                *pos += 1;
                return;
            }
            '{' => {
                let mut depth = 0;
                while *pos < src.len() {
                    match src[*pos] {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                *pos += 1;
                                return;
                            }
                        }
                        _ => {}
                    }
                    *pos += 1;
                }
                return;
            }
            _ => *pos += 1,
        }
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Parses one compound simple selector. Returns `None` for anything else
/// (combinators, attribute selectors, pseudo-classes), so callers drop it.
fn parse_selector(text: &str) -> Option<Selector> {
    let chars: Vec<char> = text.trim().chars().collect();
    if chars.is_empty() {
        return None;
    }
    let mut sel = Selector::default();
    let mut i = 0;
    let read_ident = |i: &mut usize| -> String {
        let start = *i;
        while *i < chars.len() && is_ident_char(chars[*i]) {
            *i += 1;
        }
        chars[start..*i].iter().collect()
    };

    if chars[0] == '*' {
        i = 1;
    } else if is_ident_char(chars[0]) && chars[0] != '-' {
        sel.tag = Some(read_ident(&mut i).to_ascii_lowercase());
    }
    while i < chars.len() {
        let kind = chars[i];
        i += 1;
        let ident = read_ident(&mut i);
        if ident.is_empty() {
            return None;
        }
        match kind {
            '#' => {
                if sel.id.is_some() {
                    return None;
                }
                sel.id = Some(ident);
            }
            '.' => sel.classes.push(ident),
            _ => return None, // combinator, `[`, `:`, ...
        }
    }
    Some(sel)
}

fn parse_declarations(block: &str) -> Vec<Declaration> {
    block
        .split(';')
        .filter_map(|decl| {
            let (name, value) = decl.split_once(':')?;
            let name = name.trim().to_ascii_lowercase();
            if name.is_empty() || !name.chars().all(is_ident_char) {
                return None;
            }
            let mut value = value.trim();
            let mut important = false;
            if let Some(idx) = value.to_ascii_lowercase().rfind("!important") {
                if value[idx + "!important".len()..].trim().is_empty() {
                    important = true;
                    value = value[..idx].trim();
                }
            }
            if value.is_empty() {
                return None;
            }
            Some(Declaration {
                name,
                value: parse_value(value),
                important,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Color, Unit};

    #[test]
    fn parses_rules_and_declarations() {
        let css = "h1, .note { color: red; margin: 4px 8px } /* c */ p { display: block }";
        let sheet = parse_stylesheet(css);
        assert_eq!(sheet.rules.len(), 2);
        assert_eq!(sheet.rules[0].selectors.len(), 2);
        assert_eq!(
            sheet.rules[0].declarations[0],
            Declaration {
                name: "color".into(),
                value: Value::Color(Color::rgb(255, 0, 0)),
                important: false
            }
        );
        assert_eq!(
            sheet.rules[0].declarations[1].value,
            Value::List(vec![
                Value::Length(4.0, Unit::Px),
                Value::Length(8.0, Unit::Px)
            ])
        );
    }

    #[test]
    fn parses_compound_selector_and_specificity() {
        let sel = parse_selector("div.a.b#main").unwrap();
        assert_eq!(sel.tag.as_deref(), Some("div"));
        assert_eq!(sel.id.as_deref(), Some("main"));
        assert_eq!(sel.classes, vec!["a", "b"]);
        assert_eq!(sel.specificity(), (1, 2, 1));
    }

    #[test]
    fn matching() {
        let sel = parse_selector("p.intro").unwrap();
        assert!(sel.matches("p", None, &["intro", "x"]));
        assert!(!sel.matches("div", None, &["intro"]));
        assert!(!sel.matches("p", None, &["other"]));
        assert!(parse_selector("*").unwrap().matches("anything", None, &[]));
    }

    #[test]
    fn drops_unsupported_selectors_but_keeps_the_rest() {
        let sheet = parse_stylesheet("a:hover, b > c, p { color: blue }");
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].selectors.len(), 1);
        assert_eq!(sheet.rules[0].selectors[0].tag.as_deref(), Some("p"));
    }

    #[test]
    fn skips_at_rules() {
        let css =
            "@import url(x.css); @media (min-width: 1px) { p { color: red } } b { color: blue }";
        let sheet = parse_stylesheet(css);
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].selectors[0].tag.as_deref(), Some("b"));
    }

    #[test]
    fn important_flag() {
        let sheet = parse_stylesheet("p { color: red !important; margin: 0 }");
        assert!(sheet.rules[0].declarations[0].important);
        assert!(!sheet.rules[0].declarations[1].important);
    }

    #[test]
    fn tolerates_garbage() {
        let sheet = parse_stylesheet("p { color red; : 1; margin: 1px } {{{ ");
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].declarations.len(), 1);
    }
}
