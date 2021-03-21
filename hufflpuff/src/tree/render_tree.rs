use crate::Node;
use crate::Tree;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct Edge {
    from: u16,
    to: u16,
    transition: u8, // 0 or 1
}

pub fn render_plan_to<W: core::fmt::Write>(
    graph: &Tree,
    output: &mut W,
) -> std::result::Result<(), core::fmt::Error> {
    dot::render(graph, output)
}

impl<'a> dot::Labeller<'a> for Tree {
    type Node = Node;
    type Edge = Edge;
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("huffman").unwrap()
    }

    fn node_id(&'a self, n: &Node) -> dot::Id<'a> {
        // dot::Id::new(format!("N{}", n.count)).unwrap()
        dot::Id::new(format!("N{}", self.get_node_pos(*n).unwrap())).unwrap()
    }

    fn node_label<'b>(&'b self, n: &Node) -> dot::LabelText<'b> {
        let out = if let Some(symbol) = n.symbol {
            format!("Cnt:{:?} Symbl:{:?}", n.count, symbol)
        } else {
            format!("Cnt:{:?}", n.count)
        };
        dot::LabelText::LabelStr(out.into())
    }

    /// Adds attr to `n` that will be used in the rendered output.
    /// Multiple attr can be returned in the String, e.g. `color="red", fontcolor="red"`
    fn node_attr(&'a self, n: &Self::Node) -> Option<String> {
        let out = if n.symbol.is_some() {
            "color=dodgerblue4 fontcolor=dodgerblue4 ".to_string()
        } else {
            "color=azure4 fontcolor=azure4 ".to_string()
        };
        Some(out)
    }

    fn edge_label<'b>(&'b self, ed: &Edge) -> dot::LabelText<'b> {
        dot::LabelText::LabelStr(ed.transition.to_string().into())
    }
}

impl<'a> dot::GraphWalk<'a> for Tree {
    type Node = Node;
    type Edge = Edge;
    fn nodes(&self) -> dot::Nodes<Node> {
        self.nodes
            .iter()
            .filter(|el| el.count != 0)
            .cloned()
            .collect()
    }

    fn edges(&self) -> dot::Edges<Edge> {
        let mut edges = vec![];
        for (i, node) in self.nodes.iter().enumerate() {
            if let Some(left) = node.left {
                edges.push(Edge {
                    from: i as u16,
                    to: left,
                    transition: 0,
                });
            }
            if let Some(right) = node.right {
                edges.push(Edge {
                    from: i as u16,
                    to: right,
                    transition: 1,
                });
            }
        }
        Cow::Owned(edges)
    }

    fn source(&self, e: &Edge) -> Node {
        self.nodes[e.from as usize]
    }

    fn target(&self, e: &Edge) -> Node {
        self.nodes[e.to as usize]
    }
}
