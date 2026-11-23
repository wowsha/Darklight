//! The built-in welcome page, wired through the engine crates.
//!
//! Until layout and text rendering land (Phase 1), this just proves the
//! pipeline links together on every platform and logs what it parsed.

use darklight_css::{Stylesheet, parse_stylesheet};
use darklight_html::{Document, parse};

const WELCOME_HTML: &str = include_str!("welcome.html");

pub struct Page {
    pub document: Document,
    pub stylesheet: Stylesheet,
    pub title: String,
}

impl Page {
    pub fn welcome() -> Self {
        Self::from_html(WELCOME_HTML)
    }

    pub fn from_html(html: &str) -> Self {
        let document = parse(html);
        let css = document
            .find_element("style")
            .map(|id| document.text_content(id))
            .unwrap_or_default();
        let title = document
            .find_element("title")
            .map(|id| document.text_content(id))
            .unwrap_or_else(|| "Darklight".to_string());
        Page {
            document,
            stylesheet: parse_stylesheet(&css),
            title,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_page_parses() {
        let page = Page::welcome();
        assert_eq!(page.title, "Darklight");
        assert_eq!(page.stylesheet.rules.len(), 3);
        assert!(page.document.find_element("h1").is_some());
    }
}
