//! Tree builder: tokens in, [`Document`] out.
//!
//! This follows the shape of the WHATWG tree-construction algorithm: insertion modes
//! (initial, before html, before head, in head, after head, in body, after body, after
//! after body), a stack of open elements, scope checks, implied end tags and the
//! "any other end tag" recovery rule. `html`, `head` and `body` are always created.
//!
//! Known gaps (see `docs/ROADMAP.md`, Phase 1):
//! - No active-formatting-elements list or adoption agency algorithm, so misnested
//!   `<b><i></b></i>` does not recover the way browsers do.
//! - Tables are "table-lite": table-part elements are inserted as written, with implied
//!   closing of `td`/`th`/`tr`/`tbody`, but there are no table insertion modes, no implied
//!   `tbody` and no foster parenting.
//! - `noscript` content in `<head>` is moved into `<body>`; no `template`, `select` modes,
//!   framesets, foreign content (SVG/MathML) or quirks mode.

use crate::dom::{Document, ElementData, NodeData, NodeId};
use crate::tokenizer::{Token, tokenize};

/// Maximum number of open elements. Deeper nesting is flattened instead of growing
/// without bound, so hostile pages cannot build arbitrarily deep trees (NFR-3).
pub const MAX_DEPTH: usize = 512;

const VOID: &[&str] = &[
    "area", "base", "basefont", "bgsound", "br", "col", "embed", "hr", "img", "input", "keygen",
    "link", "meta", "param", "source", "track", "wbr",
];

/// Elements for which the tokenizer emits raw text or RCDATA.
const TEXT_MODE: &[&str] = &["title", "style", "script", "textarea", "noframes"];

const SPECIAL: &[&str] = &[
    "address",
    "applet",
    "area",
    "article",
    "aside",
    "base",
    "basefont",
    "bgsound",
    "blockquote",
    "body",
    "br",
    "button",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dir",
    "div",
    "dl",
    "dt",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "iframe",
    "img",
    "input",
    "keygen",
    "li",
    "link",
    "listing",
    "main",
    "marquee",
    "menu",
    "meta",
    "nav",
    "noembed",
    "noframes",
    "noscript",
    "object",
    "ol",
    "p",
    "param",
    "plaintext",
    "pre",
    "script",
    "search",
    "section",
    "select",
    "source",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
    "wbr",
    "xmp",
];

const IMPLIED_END: &[&str] = &[
    "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc",
];

const SCOPE: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template",
];
const BUTTON_SCOPE: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "button",
];
const LIST_ITEM_SCOPE: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "ol", "ul",
];
const TABLE_SCOPE: &[&str] = &["html", "table", "template"];

const BLOCK_START: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "header",
    "hgroup",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "search",
    "section",
    "summary",
    "ul",
];

const BLOCK_END: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "button",
    "center",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "header",
    "hgroup",
    "listing",
    "main",
    "menu",
    "nav",
    "ol",
    "pre",
    "search",
    "section",
    "summary",
    "ul",
];

const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];

pub fn parse(input: &str) -> Document {
    let mut builder = Builder::new();
    for token in tokenize(input) {
        builder.process(token.into());
    }
    builder.process(Tok::Eof);
    builder.doc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    AfterBody,
    AfterAfterBody,
}

#[derive(Debug, Clone)]
enum Tok {
    Doctype(String),
    Start {
        name: String,
        attrs: Vec<(String, String)>,
    },
    End(String),
    Text(String),
    Comment(String),
    Eof,
}

impl From<Token> for Tok {
    fn from(t: Token) -> Self {
        match t {
            Token::Doctype(n) => Tok::Doctype(n),
            Token::StartTag { name, attrs, .. } => Tok::Start { name, attrs },
            Token::EndTag(n) => Tok::End(n),
            Token::Text(t) => Tok::Text(t),
            Token::Comment(c) => Tok::Comment(c),
        }
    }
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0c' | '\r')
}

/// Splits leading HTML whitespace from `s`.
fn split_leading_ws(s: &str) -> (&str, &str) {
    let rest = s.trim_start_matches(is_ws);
    (&s[..s.len() - rest.len()], rest)
}

struct Builder {
    doc: Document,
    open: Vec<NodeId>,
    mode: Mode,
    head: Option<NodeId>,
    /// A newline directly after `<pre>`/`<textarea>` is dropped.
    skip_newline: bool,
}

impl Builder {
    fn new() -> Self {
        let doc = Document::new();
        let root = doc.root();
        Builder {
            doc,
            open: vec![root],
            mode: Mode::Initial,
            head: None,
            skip_newline: false,
        }
    }

    // ---- driver -----------------------------------------------------------

    fn process(&mut self, tok: Tok) {
        let mut tok = tok;
        if std::mem::take(&mut self.skip_newline) {
            if let Tok::Text(t) = &tok {
                match t.strip_prefix('\n') {
                    Some("") => return,
                    Some(rest) => tok = Tok::Text(rest.to_string()),
                    None => {}
                }
            }
        }
        // Reprocessing only ever moves the insertion mode forward, so this terminates; the
        // cap is a safety net so a logic bug can never hang the browser.
        for _ in 0..16 {
            match self.step(tok) {
                Some(again) => tok = again,
                None => return,
            }
        }
    }

    /// Handles one token. Returns it (possibly changed) if it must be reprocessed.
    fn step(&mut self, tok: Tok) -> Option<Tok> {
        if self.in_text_mode() {
            return self.step_text_mode(tok);
        }
        match self.mode {
            Mode::Initial => self.initial(tok),
            Mode::BeforeHtml => self.before_html(tok),
            Mode::BeforeHead => self.before_head(tok),
            Mode::InHead => self.in_head(tok),
            Mode::AfterHead => self.after_head(tok),
            Mode::InBody => self.in_body(tok),
            Mode::AfterBody => self.after_body(tok),
            Mode::AfterAfterBody => self.after_after_body(tok),
        }
    }

    // ---- stack and tree helpers --------------------------------------------

    fn current(&self) -> NodeId {
        *self.open.last().expect("the stack always holds the root")
    }

    fn tag_of(&self, id: NodeId) -> Option<&str> {
        match &self.doc.node(id).data {
            NodeData::Element(e) => Some(e.tag.as_str()),
            _ => None,
        }
    }

    fn current_tag(&self) -> Option<&str> {
        self.tag_of(self.current())
    }

    fn in_text_mode(&self) -> bool {
        self.current_tag().is_some_and(|t| TEXT_MODE.contains(&t))
    }

    /// Inside `<title>`, `<style>`, `<script>`, `<textarea>`: text is inserted verbatim
    /// and the next end tag closes the element.
    fn step_text_mode(&mut self, tok: Tok) -> Option<Tok> {
        match tok {
            Tok::Text(t) => {
                let cur = self.current();
                self.doc.append_text(cur, &t);
                None
            }
            Tok::End(_) => {
                self.pop_text_element();
                None
            }
            other => {
                // Cannot happen with our tokenizer (it swallows everything up to the end
                // tag), but stay safe if it does: close the element and carry on.
                self.pop_text_element();
                Some(other)
            }
        }
    }

    fn pop_text_element(&mut self) {
        self.open.pop();
        // A head-ish element that appeared after `</head>` was inserted by temporarily
        // re-opening `<head>`; close that again.
        if self.mode == Mode::AfterHead && self.head.is_some_and(|h| h == self.current()) {
            self.open.pop();
        }
    }

    fn insert_comment(&mut self, text: String) {
        let cur = self.current();
        self.doc.append(cur, NodeData::Comment(text));
    }

    fn insert_text(&mut self, text: &str) {
        let cur = self.current();
        self.doc.append_text(cur, text);
    }

    /// Creates an element as a child of the current node. Returns its id, and whether it
    /// was pushed onto the stack of open elements.
    fn insert(&mut self, name: &str, attrs: Vec<(String, String)>) -> NodeId {
        let cur = self.current();
        self.doc.append(
            cur,
            NodeData::Element(ElementData {
                tag: name.to_string(),
                attrs,
            }),
        )
    }

    /// Inserts an element and makes it the current node (unless nesting is too deep).
    fn insert_and_push(&mut self, name: &str, attrs: Vec<(String, String)>) -> NodeId {
        let id = self.insert(name, attrs);
        if self.open.len() < MAX_DEPTH {
            self.open.push(id);
        }
        id
    }

    fn merge_attrs(&mut self, target: NodeId, attrs: Vec<(String, String)>) {
        if let Some(el) = self.doc.element_mut(target) {
            for (name, value) in attrs {
                if !el.attrs.iter().any(|(n, _)| *n == name) {
                    el.attrs.push((name, value));
                }
            }
        }
    }

    fn in_scope(&self, name: &str, boundaries: &[&str]) -> bool {
        for &id in self.open.iter().rev() {
            match self.tag_of(id) {
                Some(t) if t == name => return true,
                Some(t) if boundaries.contains(&t) => return false,
                _ => {}
            }
        }
        false
    }

    fn any_in_scope(&self, names: &[&str], boundaries: &[&str]) -> bool {
        for &id in self.open.iter().rev() {
            match self.tag_of(id) {
                Some(t) if names.contains(&t) => return true,
                Some(t) if boundaries.contains(&t) => return false,
                _ => {}
            }
        }
        false
    }

    /// Pops elements until one named `name` has been popped (inclusive).
    fn pop_until(&mut self, name: &str) {
        while let Some(id) = self.open.pop() {
            if self.tag_of(id) == Some(name) {
                return;
            }
            if self.open.is_empty() {
                // Never pop the root; restore it.
                self.open.push(id);
                return;
            }
        }
    }

    fn pop_until_any(&mut self, names: &[&str]) {
        while self.open.len() > 1 {
            let id = self.open.pop().expect("length checked");
            if self.tag_of(id).is_some_and(|t| names.contains(&t)) {
                return;
            }
        }
    }

    fn generate_implied_end_tags(&mut self, except: Option<&str>) {
        while self.open.len() > 1 {
            match self.current_tag() {
                Some(t) if IMPLIED_END.contains(&t) && Some(t) != except => {
                    self.open.pop();
                }
                _ => break,
            }
        }
    }

    fn close_p(&mut self) {
        if self.in_scope("p", BUTTON_SCOPE) {
            self.generate_implied_end_tags(Some("p"));
            self.pop_until("p");
        }
    }

    // ---- insertion modes -----------------------------------------------------

    fn initial(&mut self, tok: Tok) -> Option<Tok> {
        match tok {
            Tok::Text(t) => {
                let (_, rest) = split_leading_ws(&t);
                if rest.is_empty() {
                    return None;
                }
                self.mode = Mode::BeforeHtml;
                Some(Tok::Text(rest.to_string()))
            }
            Tok::Comment(c) => {
                let root = self.doc.root();
                self.doc.append(root, NodeData::Comment(c));
                None
            }
            Tok::Doctype(name) => {
                let root = self.doc.root();
                self.doc.append(root, NodeData::Doctype(name));
                self.mode = Mode::BeforeHtml;
                None
            }
            other => {
                self.mode = Mode::BeforeHtml;
                Some(other)
            }
        }
    }

    fn before_html(&mut self, tok: Tok) -> Option<Tok> {
        match tok {
            Tok::Text(t) => {
                let (_, rest) = split_leading_ws(&t);
                if rest.is_empty() {
                    return None;
                }
                self.create_html(Vec::new());
                Some(Tok::Text(rest.to_string()))
            }
            Tok::Comment(c) => {
                let root = self.doc.root();
                self.doc.append(root, NodeData::Comment(c));
                None
            }
            Tok::Doctype(_) => None,
            Tok::Start { name, attrs } if name == "html" => {
                self.create_html(attrs);
                None
            }
            Tok::End(name) if !matches!(name.as_str(), "head" | "body" | "html" | "br") => None,
            other => {
                self.create_html(Vec::new());
                Some(other)
            }
        }
    }

    fn create_html(&mut self, attrs: Vec<(String, String)>) {
        self.insert_and_push("html", attrs);
        self.mode = Mode::BeforeHead;
    }

    fn before_head(&mut self, tok: Tok) -> Option<Tok> {
        match tok {
            Tok::Text(t) => {
                let (_, rest) = split_leading_ws(&t);
                if rest.is_empty() {
                    return None;
                }
                self.create_head(Vec::new());
                Some(Tok::Text(rest.to_string()))
            }
            Tok::Comment(c) => {
                self.insert_comment(c);
                None
            }
            Tok::Doctype(_) => None,
            Tok::Start { name, attrs } if name == "html" => {
                self.merge_into_html(attrs);
                None
            }
            Tok::Start { name, attrs } if name == "head" => {
                self.create_head(attrs);
                None
            }
            Tok::End(name) if !matches!(name.as_str(), "head" | "body" | "html" | "br") => None,
            other => {
                self.create_head(Vec::new());
                Some(other)
            }
        }
    }

    fn create_head(&mut self, attrs: Vec<(String, String)>) {
        let id = self.insert_and_push("head", attrs);
        self.head = Some(id);
        self.mode = Mode::InHead;
    }

    fn merge_into_html(&mut self, attrs: Vec<(String, String)>) {
        if let Some(&html) = self.open.get(1) {
            self.merge_attrs(html, attrs);
        }
    }

    fn in_head(&mut self, tok: Tok) -> Option<Tok> {
        match tok {
            Tok::Text(t) => {
                let (ws, rest) = split_leading_ws(&t);
                self.insert_text(ws);
                if rest.is_empty() {
                    return None;
                }
                self.leave_head();
                Some(Tok::Text(rest.to_string()))
            }
            Tok::Comment(c) => {
                self.insert_comment(c);
                None
            }
            Tok::Doctype(_) => None,
            Tok::Start { name, attrs } => match name.as_str() {
                "html" => {
                    self.merge_into_html(attrs);
                    None
                }
                "head" => None,
                "base" | "basefont" | "bgsound" | "link" | "meta" | "title" | "style"
                | "script" | "noframes" => {
                    self.head_element(&name, attrs);
                    None
                }
                _ => {
                    self.leave_head();
                    Some(Tok::Start { name, attrs })
                }
            },
            Tok::End(name) => match name.as_str() {
                "head" => {
                    self.leave_head();
                    None
                }
                "body" | "html" | "br" => {
                    self.leave_head();
                    Some(Tok::End(name))
                }
                _ => None,
            },
            Tok::Eof => {
                self.leave_head();
                Some(Tok::Eof)
            }
        }
    }

    /// Inserts a head-content element at the current node: void ones are just inserted,
    /// the rest become the current node so their text is collected.
    fn head_element(&mut self, name: &str, attrs: Vec<(String, String)>) {
        if VOID.contains(&name) {
            self.insert(name, attrs);
        } else {
            self.insert_and_push(name, attrs);
        }
    }

    fn leave_head(&mut self) {
        if self.current_tag() == Some("head") {
            self.open.pop();
        }
        self.mode = Mode::AfterHead;
    }

    fn after_head(&mut self, tok: Tok) -> Option<Tok> {
        match tok {
            Tok::Text(t) => {
                let (ws, rest) = split_leading_ws(&t);
                self.insert_text(ws);
                if rest.is_empty() {
                    return None;
                }
                self.create_body(Vec::new());
                Some(Tok::Text(rest.to_string()))
            }
            Tok::Comment(c) => {
                self.insert_comment(c);
                None
            }
            Tok::Doctype(_) => None,
            Tok::Start { name, attrs } => match name.as_str() {
                "html" => {
                    self.merge_into_html(attrs);
                    None
                }
                "body" => {
                    self.create_body(attrs);
                    None
                }
                "head" => None,
                "base" | "basefont" | "bgsound" | "link" | "meta" | "title" | "style"
                | "script" | "noframes" => {
                    // Head content after `</head>` still belongs in the head.
                    if let Some(head) = self.head {
                        self.open.push(head);
                        let before = self.open.len();
                        self.head_element(&name, attrs);
                        if self.open.len() == before {
                            self.open.pop(); // void element: close the re-opened head
                        }
                    }
                    None
                }
                _ => {
                    self.create_body(Vec::new());
                    Some(Tok::Start { name, attrs })
                }
            },
            Tok::End(name) => match name.as_str() {
                "body" | "html" | "br" => {
                    self.create_body(Vec::new());
                    Some(Tok::End(name))
                }
                _ => None,
            },
            Tok::Eof => {
                self.create_body(Vec::new());
                Some(Tok::Eof)
            }
        }
    }

    fn create_body(&mut self, attrs: Vec<(String, String)>) {
        self.insert_and_push("body", attrs);
        self.mode = Mode::InBody;
    }

    fn in_body(&mut self, tok: Tok) -> Option<Tok> {
        match tok {
            Tok::Text(t) => {
                self.insert_text(&t);
                None
            }
            Tok::Comment(c) => {
                self.insert_comment(c);
                None
            }
            Tok::Doctype(_) | Tok::Eof => None,
            Tok::Start { name, attrs } => {
                self.body_start(name, attrs);
                None
            }
            Tok::End(name) => self.body_end(name),
        }
    }

    fn body_start(&mut self, name: String, attrs: Vec<(String, String)>) {
        let n = name.as_str();
        match n {
            "html" => self.merge_into_html(attrs),
            "body" => {
                if let Some(&body) = self.open.get(2) {
                    if self.tag_of(body) == Some("body") {
                        self.merge_attrs(body, attrs);
                    }
                }
            }
            "head" => {}
            "base" | "basefont" | "bgsound" | "link" | "meta" | "title" | "style" | "script"
            | "noframes" => self.head_element(n, attrs),
            _ if BLOCK_START.contains(&n) => {
                self.close_p();
                self.insert_and_push(n, attrs);
            }
            _ if HEADINGS.contains(&n) => {
                self.close_p();
                if self.current_tag().is_some_and(|t| HEADINGS.contains(&t)) {
                    self.open.pop();
                }
                self.insert_and_push(n, attrs);
            }
            "pre" | "listing" => {
                self.close_p();
                self.insert_and_push(n, attrs);
                self.skip_newline = true;
            }
            "form" => {
                self.close_p();
                self.insert_and_push(n, attrs);
            }
            "li" => {
                self.close_open_list_item(&["li"]);
                self.close_p();
                self.insert_and_push(n, attrs);
            }
            "dd" | "dt" => {
                self.close_open_list_item(&["dd", "dt"]);
                self.close_p();
                self.insert_and_push(n, attrs);
            }
            "button" => {
                if self.in_scope("button", SCOPE) {
                    self.generate_implied_end_tags(None);
                    self.pop_until("button");
                }
                self.insert_and_push(n, attrs);
            }
            "option" | "optgroup" => {
                if self.current_tag() == Some("option") {
                    self.open.pop();
                }
                self.insert_and_push(n, attrs);
            }
            "hr" => {
                self.close_p();
                self.insert(n, attrs);
            }
            "textarea" => {
                self.insert_and_push(n, attrs);
                self.skip_newline = true;
            }
            "table" => {
                self.close_p();
                self.insert_and_push(n, attrs);
            }
            "td" | "th" => {
                self.close_table_part(&["td", "th"]);
                self.insert_and_push(n, attrs);
            }
            "tr" => {
                self.close_table_part(&["tr"]);
                self.insert_and_push(n, attrs);
            }
            "thead" | "tbody" | "tfoot" => {
                self.close_table_part(&["thead", "tbody", "tfoot"]);
                self.insert_and_push(n, attrs);
            }
            "image" => {
                self.insert("img", attrs);
            }
            _ if VOID.contains(&n) => {
                self.insert(n, attrs);
            }
            _ => {
                self.insert_and_push(n, attrs);
            }
        }
    }

    /// The `li` / `dd` / `dt` start-tag loop: close an open item of the same kind, looking
    /// past ordinary elements but stopping at other "special" ones.
    fn close_open_list_item(&mut self, names: &[&str]) {
        let mut found = None;
        for &id in self.open.iter().rev() {
            let Some(tag) = self.tag_of(id) else { break };
            if names.contains(&tag) {
                found = Some(tag.to_string());
                break;
            }
            if SPECIAL.contains(&tag) && !matches!(tag, "address" | "div" | "p") {
                break;
            }
        }
        if let Some(tag) = found {
            self.generate_implied_end_tags(Some(&tag));
            self.pop_until(&tag);
        }
    }

    /// Closes the nearest open element among `names`, without leaving the current table
    /// (so an open `td` in an outer table is not closed by a `td` in a nested one).
    fn close_table_part(&mut self, names: &[&str]) {
        if self.any_in_scope(names, TABLE_SCOPE) {
            self.pop_until_any(names);
        }
    }

    fn body_end(&mut self, name: String) -> Option<Tok> {
        let n = name.as_str();
        match n {
            "body" => {
                if self.in_scope("body", SCOPE) {
                    self.mode = Mode::AfterBody;
                }
                None
            }
            "html" => {
                if self.in_scope("body", SCOPE) {
                    self.mode = Mode::AfterBody;
                    return Some(Tok::End(name));
                }
                None
            }
            _ if BLOCK_END.contains(&n) => {
                if self.in_scope(n, SCOPE) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(n);
                }
                None
            }
            "p" => {
                if self.in_scope("p", BUTTON_SCOPE) {
                    self.generate_implied_end_tags(Some("p"));
                    self.pop_until("p");
                } else {
                    // `</p>` with no open paragraph creates an empty one.
                    self.insert("p", Vec::new());
                }
                None
            }
            "li" => {
                if self.in_scope("li", LIST_ITEM_SCOPE) {
                    self.generate_implied_end_tags(Some("li"));
                    self.pop_until("li");
                }
                None
            }
            "dd" | "dt" => {
                if self.in_scope(n, SCOPE) {
                    self.generate_implied_end_tags(Some(n));
                    self.pop_until(n);
                }
                None
            }
            _ if HEADINGS.contains(&n) => {
                if self.any_in_scope(HEADINGS, SCOPE) {
                    self.generate_implied_end_tags(None);
                    self.pop_until_any(HEADINGS);
                }
                None
            }
            "td" | "th" | "tr" | "thead" | "tbody" | "tfoot" | "caption" | "colgroup" => {
                if self.in_scope(n, TABLE_SCOPE) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(n);
                }
                None
            }
            "table" => {
                if self.in_scope("table", &["html", "template"]) {
                    self.generate_implied_end_tags(None);
                    self.pop_until("table");
                }
                None
            }
            "br" => {
                self.insert("br", Vec::new());
                None
            }
            _ => {
                self.any_other_end_tag(n);
                None
            }
        }
    }

    /// The generic end-tag rule: close the nearest matching open element, unless a
    /// "special" element is in the way, in which case the end tag is ignored.
    fn any_other_end_tag(&mut self, name: &str) {
        let mut target = None;
        for (i, &id) in self.open.iter().enumerate().rev() {
            match self.tag_of(id) {
                Some(t) if t == name => {
                    target = Some(i);
                    break;
                }
                Some(t) if SPECIAL.contains(&t) => return,
                _ => {}
            }
        }
        if let Some(i) = target {
            // Implied end tags are generated for elements above the target.
            while self.open.len() > i + 1 {
                self.open.pop();
            }
            self.open.pop();
        }
    }

    fn after_body(&mut self, tok: Tok) -> Option<Tok> {
        match tok {
            Tok::Text(t) => {
                let (ws, rest) = split_leading_ws(&t);
                if !ws.is_empty() {
                    self.insert_text(ws);
                }
                if rest.is_empty() {
                    return None;
                }
                self.mode = Mode::InBody;
                Some(Tok::Text(rest.to_string()))
            }
            Tok::Comment(c) => {
                // Comments after `</body>` attach to the `<html>` element.
                if let Some(&html) = self.open.get(1) {
                    self.doc.append(html, NodeData::Comment(c));
                }
                None
            }
            Tok::Doctype(_) => None,
            Tok::End(name) if name == "html" => {
                self.mode = Mode::AfterAfterBody;
                None
            }
            Tok::Eof => None,
            other => {
                self.mode = Mode::InBody;
                Some(other)
            }
        }
    }

    fn after_after_body(&mut self, tok: Tok) -> Option<Tok> {
        match tok {
            Tok::Comment(c) => {
                let root = self.doc.root();
                self.doc.append(root, NodeData::Comment(c));
                None
            }
            Tok::Doctype(_) | Tok::Eof => None,
            Tok::Text(t) => {
                let (ws, rest) = split_leading_ws(&t);
                if !ws.is_empty() {
                    self.insert_text(ws);
                }
                if rest.is_empty() {
                    return None;
                }
                self.mode = Mode::InBody;
                Some(Tok::Text(rest.to_string()))
            }
            other => {
                self.mode = Mode::InBody;
                Some(other)
            }
        }
    }
}

#[cfg(test)]
mod tests;
