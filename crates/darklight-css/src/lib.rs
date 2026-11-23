//! CSS parsing for Darklight.
//!
//! Deliberately small: simple selectors (type, `*`, `#id`, `.class`, and
//! compounds of those), declarations with colors, lengths and keywords.
//! Combinators, attribute selectors, pseudo-classes and at-rules are skipped
//! (at-rules) or dropped (unsupported selectors) rather than mis-parsed.
//! See `docs/ROADMAP.md` Phase 1 for the move to a CSS Syntax Level 3 tokenizer.

#![forbid(unsafe_code)]

mod parser;
mod value;

pub use parser::{Declaration, Rule, Selector, Specificity, Stylesheet, parse_stylesheet};
pub use value::{Color, Unit, Value, parse_value};
