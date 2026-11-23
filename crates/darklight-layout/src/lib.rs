//! Layout primitives for Darklight: rectangles and the CSS box model.
//!
//! Block and inline formatting contexts land in Phase 1 (see `docs/ROADMAP.md`).

#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn expanded_by(self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// The CSS box model for one box: content area plus padding, border and margin.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    pub fn padding_box(&self) -> Rect {
        self.content.expanded_by(self.padding)
    }

    pub fn border_box(&self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }

    pub fn margin_box(&self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_model_expands_outward() {
        let d = Dimensions {
            content: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
            },
            padding: EdgeSizes {
                left: 1.0,
                right: 2.0,
                top: 3.0,
                bottom: 4.0,
            },
            border: EdgeSizes {
                left: 1.0,
                right: 1.0,
                top: 1.0,
                bottom: 1.0,
            },
            margin: EdgeSizes {
                left: 5.0,
                right: 5.0,
                top: 5.0,
                bottom: 5.0,
            },
        };
        assert_eq!(
            d.padding_box(),
            Rect {
                x: 9.0,
                y: 7.0,
                width: 103.0,
                height: 57.0
            }
        );
        assert_eq!(
            d.border_box(),
            Rect {
                x: 8.0,
                y: 6.0,
                width: 105.0,
                height: 59.0
            }
        );
        assert_eq!(
            d.margin_box(),
            Rect {
                x: 3.0,
                y: 1.0,
                width: 115.0,
                height: 69.0
            }
        );
    }
}
