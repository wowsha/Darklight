//! A minimal arena-based DOM.

/// Index of a node inside a [`Document`] arena.
pub type NodeId = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeData {
    Document,
    Doctype(String),
    Element(ElementData),
    Text(String),
    Comment(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ElementData {
    /// Lower-cased tag name.
    pub tag: String,
    /// Attributes in source order. Names are lower-cased.
    pub attrs: Vec<(String, String)>,
}

impl ElementData {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn id(&self) -> Option<&str> {
        self.attr("id")
    }

    pub fn classes(&self) -> Vec<&str> {
        self.attr("class")
            .map(|c| c.split_ascii_whitespace().collect())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub data: NodeData,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct Document {
    nodes: Vec<Node>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    pub const ROOT: NodeId = 0;

    pub fn new() -> Self {
        Self {
            nodes: vec![Node {
                data: NodeData::Document,
                parent: None,
                children: Vec::new(),
            }],
        }
    }

    pub fn root(&self) -> NodeId {
        Self::ROOT
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        false // a document always contains its root node
    }

    /// Appends a new node as the last child of `parent` and returns its id.
    pub fn append(&mut self, parent: NodeId, data: NodeData) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node {
            data,
            parent: Some(parent),
            children: Vec::new(),
        });
        self.nodes[parent].children.push(id);
        id
    }

    /// Depth-first, document-order list of all nodes below (and including) `start`.
    pub fn descendants(&self, start: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![start];
        while let Some(id) = stack.pop() {
            out.push(id);
            stack.extend(self.nodes[id].children.iter().rev());
        }
        out
    }

    /// Concatenated text of all text nodes below `start`.
    pub fn text_content(&self, start: NodeId) -> String {
        let mut s = String::new();
        for id in self.descendants(start) {
            if let NodeData::Text(t) = &self.nodes[id].data {
                s.push_str(t);
            }
        }
        s
    }

    /// First element (document order) with the given tag name.
    pub fn find_element(&self, tag: &str) -> Option<NodeId> {
        self.descendants(Self::ROOT)
            .into_iter()
            .find(|&id| matches!(&self.nodes[id].data, NodeData::Element(e) if e.tag == tag))
    }
}
