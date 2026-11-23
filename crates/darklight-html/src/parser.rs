//! A simplified tree builder: tokens in, [`Document`] out.
//!
//! Supported: void elements, implied end tags for `p`, `li`, `dt`/`dd`, and
//! recovery from stray end tags. Not yet supported: implied `html`/`head`/`body`,
//! foster parenting, the adoption agency algorithm, namespaces.

use crate::dom::{Document, ElementData, NodeData, NodeId};
use crate::tokenizer::{Token, tokenize};

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Start tags that implicitly close an open `<p>`.
const CLOSES_P: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "div",
    "dl",
    "fieldset",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "ul",
];

pub fn parse(input: &str) -> Document {
    let mut doc = Document::new();
    let mut open: Vec<NodeId> = vec![doc.root()];

    for token in tokenize(input) {
        let current = *open.last().expect("stack always contains the root");
        match token {
            Token::Doctype(name) => {
                doc.append(doc.root(), NodeData::Doctype(name));
            }
            Token::Comment(text) => {
                doc.append(current, NodeData::Comment(text));
            }
            Token::Text(text) => {
                // Ignore whitespace between top-level nodes.
                if current == doc.root() && text.trim().is_empty() {
                    continue;
                }
                doc.append(current, NodeData::Text(text));
            }
            Token::StartTag { name, attrs, .. } => {
                close_implied(&doc, &mut open, &name);
                let parent = *open.last().expect("stack always contains the root");
                let is_void = VOID.contains(&name.as_str());
                let id = doc.append(parent, NodeData::Element(ElementData { tag: name, attrs }));
                if !is_void {
                    open.push(id);
                }
            }
            Token::EndTag(name) => {
                // Pop up to and including the nearest matching open element.
                // A stray end tag with no match is ignored.
                let found = open.iter().rposition(|&id| {
                    id != doc.root()
                        && matches!(&doc.node(id).data, NodeData::Element(e) if e.tag == name)
                });
                if let Some(pos) = found {
                    open.truncate(pos);
                }
            }
        }
    }
    doc
}

fn current_tag<'a>(doc: &'a Document, open: &[NodeId]) -> Option<&'a str> {
    match &doc.node(*open.last()?).data {
        NodeData::Element(e) => Some(e.tag.as_str()),
        _ => None,
    }
}

fn close_implied(doc: &Document, open: &mut Vec<NodeId>, incoming: &str) {
    let cur = current_tag(doc, open);
    let should_close = match (cur, incoming) {
        (Some("p"), tag) => CLOSES_P.contains(&tag),
        (Some("li"), "li") => true,
        (Some("dt" | "dd"), "dt" | "dd") => true,
        _ => false,
    };
    if should_close {
        open.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(doc: &Document, id: NodeId) -> Vec<String> {
        doc.node(id)
            .children
            .iter()
            .filter_map(|&c| match &doc.node(c).data {
                NodeData::Element(e) => Some(e.tag.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn builds_nested_structure() {
        let doc = parse("<html><body><h1>Hi</h1><p>there</p></body></html>");
        let body = doc.find_element("body").unwrap();
        assert_eq!(tags(&doc, body), vec!["h1", "p"]);
        assert_eq!(doc.text_content(body), "Hithere");
    }

    #[test]
    fn void_elements_have_no_children() {
        let doc = parse("<p>a<br>b<img src=x.png>c</p>");
        let p = doc.find_element("p").unwrap();
        assert_eq!(tags(&doc, p), vec!["br", "img"]);
        assert_eq!(doc.text_content(p), "abc");
    }

    #[test]
    fn paragraphs_close_implicitly() {
        let doc = parse("<div><p>one<p>two<div>three</div></div>");
        let div = doc.find_element("div").unwrap();
        assert_eq!(tags(&doc, div), vec!["p", "p", "div"]);
    }

    #[test]
    fn list_items_close_implicitly() {
        let doc = parse("<ul><li>a<li>b<li>c</ul>");
        let ul = doc.find_element("ul").unwrap();
        assert_eq!(tags(&doc, ul), vec!["li", "li", "li"]);
    }

    #[test]
    fn stray_end_tags_are_ignored() {
        let doc = parse("<div>a</span>b</div>");
        let div = doc.find_element("div").unwrap();
        assert_eq!(doc.text_content(div), "ab");
    }

    #[test]
    fn unclosed_elements_are_tolerated() {
        let doc = parse("<div><span>text");
        assert_eq!(doc.text_content(doc.root()), "text");
    }

    #[test]
    fn style_content_is_a_single_text_node() {
        let doc = parse("<style>p > a { color: red }</style>");
        let style = doc.find_element("style").unwrap();
        assert_eq!(doc.node(style).children.len(), 1);
        assert_eq!(doc.text_content(style), "p > a { color: red }");
    }
}
