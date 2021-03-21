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
