pub mod build_tree;
pub mod render_tree;
mod tree;
pub(crate) mod tree_node;
pub use build_tree::build_tree_fast;

pub use tree::Tree;

/// we can calculate the minimum depth of a huffman tree, by its binary tree properties.
/// A symbol is always a leaf (to uphold the prefix characteristic), therefore the maximum number of symbols is 2^depth (perfectly balanced tree)
#[inline]
pub fn minimum_tree_depth(num_symbols: usize) -> usize {
    let min_depth = (num_symbols as f32).log(2.0).ceil() as usize;
    min_depth.max(1)
}

#[test]
fn test_minimum_depth() {
    assert_eq!(minimum_tree_depth(0), 1);
    assert_eq!(minimum_tree_depth(1), 1);
    assert_eq!(minimum_tree_depth(2), 1);
    assert_eq!(minimum_tree_depth(3), 2);
    assert_eq!(minimum_tree_depth(4), 2);
    assert_eq!(minimum_tree_depth(5), 3);
    assert_eq!(minimum_tree_depth(6), 3);
    assert_eq!(minimum_tree_depth(7), 3);
    assert_eq!(minimum_tree_depth(8), 3);
    assert_eq!(minimum_tree_depth(9), 4);
}

/// we can calculate the maximum numer of nodes of a huffman tree for a depth.
/// The maimum number of nodes are in a perfectly balanced tree. 2^n for each depth
#[inline]
pub fn maximum_number_of_nodes(depth: u32) -> usize {
    (0..=depth).map(|d| 2_usize.pow(d)).sum()
}

#[test]
fn test_maximum_number_of_nodes() {
    assert_eq!(maximum_number_of_nodes(0), 1);
    assert_eq!(maximum_number_of_nodes(1), 3);
    assert_eq!(maximum_number_of_nodes(2), 3 + 4);
    assert_eq!(maximum_number_of_nodes(3), 7 + 8);
}
