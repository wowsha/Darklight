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

    /// Appends `text` to `parent`, merging into a trailing text node if there is one.
    pub fn append_text(&mut self, parent: NodeId, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(&last) = self.nodes[parent].children.last() {
            if let NodeData::Text(existing) = &mut self.nodes[last].data {
                existing.push_str(text);
                return;
            }
        }
        self.append(parent, NodeData::Text(text.to_string()));
    }

    /// Mutable access to an element's data, or `None` if `id` is not an element.
    pub fn element_mut(&mut self, id: NodeId) -> Option<&mut ElementData> {
        match &mut self.nodes[id].data {
            NodeData::Element(e) => Some(e),
            _ => None,
        }
    }

    /// Serializes the tree in the html5lib tree-construction test format, so the parser
    /// can be checked against that corpus. Attributes are sorted by name.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        let mut stack: Vec<(NodeId, usize)> = self.nodes[Self::ROOT]
            .children
            .iter()
            .rev()
            .map(|&c| (c, 0))
            .collect();
        while let Some((id, depth)) = stack.pop() {
            let pad = "  ".repeat(depth);
            let node = &self.nodes[id];
            match &node.data {
                NodeData::Document => continue,
                NodeData::Doctype(name) => out.push_str(&format!("| {pad}<!DOCTYPE {name}>\n")),
                NodeData::Comment(text) => out.push_str(&format!("| {pad}<!-- {text} -->\n")),
                NodeData::Text(text) => out.push_str(&format!("| {pad}\"{text}\"\n")),
                NodeData::Element(e) => {
                    out.push_str(&format!("| {pad}<{}>\n", e.tag));
                    let mut attrs: Vec<&(String, String)> = e.attrs.iter().collect();
                    attrs.sort();
                    for (name, value) in attrs {
                        out.push_str(&format!("| {pad}  {name}=\"{value}\"\n"));
                    }
                }
            }
            stack.extend(node.children.iter().rev().map(|&c| (c, depth + 1)));
        }
        out
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
