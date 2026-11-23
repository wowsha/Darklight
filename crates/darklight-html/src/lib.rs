//! HTML parsing and the DOM for Darklight.
//!
//! This is an intentionally small, from-scratch implementation. It is **not** yet
//! conformant with the WHATWG HTML tree-construction algorithm; see `docs/ROADMAP.md`
//! (Phase 1) for the plan to get there.

#![forbid(unsafe_code)]

pub mod dom;
pub mod parser;
pub mod tokenizer;

pub use dom::{Document, ElementData, Node, NodeData, NodeId};
pub use parser::parse;
pub use tokenizer::{Token, tokenize};
