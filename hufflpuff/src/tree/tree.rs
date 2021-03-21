use super::tree_node::Node;
use crate::tree::render_tree::render_plan_to;

#[derive(Debug)]
pub struct Tree {
    pub(crate) nodes: Vec<Node>,
    /// the root node of the tree is a parent, never a symbol. parents are at the end of the nodes vec
    /// Therefore root node is always > u8::MAX
    pub(crate) root_node: usize,

    /// the nodes with the symbols are at the beginning of the tree
    /// `last_symbol_node_pos` indicate position of the last symbol node. last_symbol_node_pos is thereforea lways <= u8::MAX
    pub(crate) last_symbol_node_pos: u8,
}

impl Tree {
    pub(crate) fn get_node_pos(&self, node: Node) -> Option<usize> {
        self.nodes.iter().position(|&r| r == node)
    }
    pub fn get_node(&self, node_pos: u16) -> &Node {
        &self.nodes[node_pos as usize]
    }
    pub fn get_depth(&self) -> u8 {
        self.nodes[0].number_bits
    }
    pub fn get_num_symbol_nodes(&self) -> u16 {
        self.last_symbol_node_pos as u16 + 1
    }
    pub fn get_root_node(&self) -> &Node {
        &self.nodes[self.root_node]
    }

    /// returns all nodes in the tree containing a symbol, excluding intermediate parent nodes
    pub fn get_symbol_nodes(&self) -> &[Node] {
        &self.nodes[..=self.last_symbol_node_pos as usize]
    }
    pub fn get_symbol_nodes_mut(&mut self) -> &mut[Node] {
        &mut self.nodes[..=self.last_symbol_node_pos as usize]
    }

    /// returns estimated compressed size in byte
    pub fn estimate_compressed_size(&self) -> usize {
        let mut size_in_bits = 0;
        for node in self.get_symbol_nodes() {
            size_in_bits += node.count as usize * node.number_bits as usize;
        }
        ((size_in_bits as f32) / 8.0).ceil() as usize
    }

    pub(crate) fn walk_tree<F>(&self, start_node_pos: usize, fun: &mut F)
    where
        F: FnMut(&Node, usize, usize),
    {
        self.walk_graph_internal(&mut 0, &mut 0, start_node_pos, fun);
    }
    fn walk_graph_internal<F>(
        &self,
        depth: &mut usize,
        transitions: &mut usize,
        start_node_pos: usize,
        fun: &mut F,
    ) where
        F: FnMut(&Node, usize, usize),
    {
        if let Some(left) = self.nodes[start_node_pos].left {
            let left_node = &self.nodes[left as usize];
            *depth += 1;
            *transitions <<= 1;
            fun(left_node, *transitions, *depth);
            self.walk_graph_internal(depth, transitions, left as usize, fun);
            *depth -= 1;
            *transitions >>= 1;
        }
        if let Some(right) = self.nodes[start_node_pos].right {
            let right_node = &self.nodes[right as usize];
            *depth += 1;
            *transitions <<= 1;
            *transitions |= 1;
            fun(right_node, *transitions, *depth);
            self.walk_graph_internal(depth, transitions, right as usize, fun);
            *transitions >>= 1;
            *depth -= 1;
        }
    }
}

impl std::fmt::Display for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        render_plan_to(&self, f)
    }
}
