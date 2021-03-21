use core::{panic};

use crate::tree::minimum_tree_depth;
use crate::tree::tree_node::MinNode;
use crate::tree::Tree;
use crate::Node;

/// creates a huffman tree and limits its height to 11
#[inline]
pub fn build_tree_fast(counts: &[usize; 256]) -> Tree {
    let mut tree = build_tree_fast_1(counts);

    let max_height = 11;
    set_max_height(&mut tree, max_height);

    tree_to_table(&tree);
    tree
}

/// creates a huffman tree
#[inline]
pub fn build_tree_fast_1(counts: &[usize; 256]) -> Tree {
    let mut nodes = [Node::default(); 512];
    let last_symbol_node_pos = {
        let mut pos: u16 = 0;
        // let mut tree = Tree::new();
        for ((byte, count), node) in counts
            .iter()
            .enumerate()
            .filter(|(_, count)| **count != 0)
            .zip(nodes.iter_mut())
        {
            *node = Node {
                symbol: Some(byte as u8),
                count: *count as u32,
                ..Default::default()
            };
            pos += 1;
        }
        (pos - 1) as u8
    };
    // sort all nodes with a symbol ASC by count
    nodes[..=last_symbol_node_pos as usize].sort_by_key(|el| el.count);
    let mut pos: usize = 0;

    // put intermediate nodes at the end
    let mut parent_check_pos = nodes.len() - 1;
    let mut parent_create_pos = nodes.len() - 1;

    // walking upwards until all symbols have a parent
    while pos <= last_symbol_node_pos as usize {

        // get the next to lowest nodes and build a parent
        let node1_pos = {
            // TODO: Check what is better nodes[pos].count < nodes[parent_check_pos].count or  nodes[pos].count <= nodes[parent_check_pos].count
            if nodes[pos].count < nodes[parent_check_pos].count
                || nodes[parent_check_pos].count == 0
            {
                increment_return_old(&mut pos)
            } else {
                decrement_return_old(&mut parent_check_pos)
            }
        };
        let node2_pos = {
            if nodes[pos].count < nodes[parent_check_pos].count
                || nodes[parent_check_pos].count == 0
            {
                increment_return_old(&mut pos)
            } else {
                decrement_return_old(&mut parent_check_pos)
            }
        };
        debug_assert!(nodes[node1_pos as usize].count != 0);

        nodes[parent_create_pos] = Node {
            count: nodes[node1_pos as usize].count + nodes[node2_pos as usize].count,
            ..Default::default()
        };

        nodes[parent_create_pos].left = Some(node1_pos as u16);
        if nodes[node2_pos as usize].count != 0 {
            nodes[parent_create_pos].right = Some(node2_pos as u16);
        } else if nodes[parent_check_pos].count != 0 {
            // could be part of the node2_pos check above, but it is only valid in the last case
            let connect_parent = decrement_return_old(&mut parent_check_pos);
            nodes[parent_create_pos].right = Some(connect_parent as u16);
            nodes[parent_create_pos].count += nodes[connect_parent].count;
        }
        parent_create_pos -= 1;
    }

    // finish unconnected parents
    while parent_check_pos - 1 > parent_create_pos {
        let node1_pos = decrement_return_old(&mut parent_check_pos);
        let node2_pos = decrement_return_old(&mut parent_check_pos);
        if nodes[node1_pos as usize].count == 0 || nodes[node2_pos as usize].count == 0 {
            break;
        }
        nodes[parent_create_pos] = Node {
            count: nodes[node1_pos as usize].count + nodes[node2_pos as usize].count,
            left: Some(node1_pos as u16),
            right: Some(node2_pos as u16),
            ..Default::default()
        };
        parent_create_pos -= 1;
    }
    let root_node = parent_create_pos + 1;
    debug_assert!(nodes[root_node].count != 0);

    // tree.walk_tree(tree.root_node, &mut |node, _transitions, depth|{
    //     node.number_bits = depth as u8;
    // });

    // we can just walk over the parents to assign number of bits, because the nodes at the beginning are only symbol leafs
    // and the parents are strictly increasing, since the algorithm creates them ordered
    for parent_node_pos in root_node..nodes.len() {
        let parent_node = nodes[parent_node_pos];
        if let Some(left) = parent_node.left {
            nodes[left as usize].number_bits = parent_node.number_bits + 1
        }
        if let Some(right) = parent_node.right {
            nodes[right as usize].number_bits = parent_node.number_bits + 1
        }
    }

    let tree = Tree {
        nodes: nodes.to_vec(),
        root_node,
        last_symbol_node_pos,
    };
    tree
}

/// converts the tree into a table with prefixes for each symbol
pub fn tree_to_table(tree: &Tree) -> [MinNode; 256] {
    let mut num_nodes_per_depth = [0_u16; 16];
    let mut node_values_per_depth = [0_u16; 16];
    for node in tree.get_symbol_nodes() {
        num_nodes_per_depth[node.number_bits as usize] += 1;
    }

    let max_depth = num_nodes_per_depth
        .iter()
        .enumerate()
        .filter(|(_depth, num_nodes)| **num_nodes != 0)
        .map(|(depth, _num_nodes)| depth)
        .last()
        .unwrap();

    // assign start values, starting at end of tree
    // this assignment is ported from FiniteStateEntropy (huf_compress.c)
    // assigning min values ensure to generate correct prefix codes. last tree always has at least two nodes
    //
    let mut min = 0;
    for depth in (0..=max_depth).rev() {
        node_values_per_depth[depth] = min;
        min += num_nodes_per_depth[depth] as u16;
        // shift out one bit
        // not completely sure where it comes from. maybe the differentiator bit to move to the next depth (depth == num bits)?
        min >>= 1;
    }

    let mut symbol_lookup_table = [MinNode::default(); u8::MAX as usize + 1];
    for node in tree.get_symbol_nodes() {
        if let Some(byte) = node.symbol {
            symbol_lookup_table[byte as usize].val =
                node_values_per_depth[node.number_bits as usize];
            symbol_lookup_table[byte as usize].number_bits = node.number_bits;
            node_values_per_depth[node.number_bits as usize] += 1;
        }
    }

    // dbg!(&num_nodes_per_depth[..10]);
    // dbg!(&symbol_lookup_table[..10]);

    symbol_lookup_table
}


/// Limits  depth of the tree
/// Note that the parents are not updated for performance reason and therefore incorrect
#[inline]
pub fn set_max_height(tree: &mut Tree, max_bits: u8) {
    assert!(minimum_tree_depth(tree.get_num_symbol_nodes() as usize) <= max_bits as usize);
    let largest_bits = tree.get_depth();
    if largest_bits <= max_bits {
        return;
    }
    let base_cost = 1 << (largest_bits - max_bits);
    let mut debt = 0_i32;
    for node in tree.get_symbol_nodes_mut() {
        if node.number_bits > max_bits {
            // let diff = node.number_bits - max_bits - 1;
            let diff = largest_bits - node.number_bits;
            debt += base_cost - (1 << diff);
            node.number_bits = max_bits;
        }
    }

    debt >>= largest_bits - max_bits;  // debt is a multiple of base_cost

    // build index of depth start and end node positions for each depth
    let mut depth_index = get_depth_index(&tree.nodes);
    let mut check_depth = max_bits - 1;
    // fix tree
    while debt > 0 && check_depth != 0 {
        // check lower number_bits to pay debt
        if depth_index[check_depth as usize].is_empty() {
            check_depth -= 1;
            continue;
        } else {
            // TODO instead moving the symbol down, the cost of two nodes here could be compared with one node on
            // the higher level, in order to generate a more optimal tree 

            // demote symbol in tree by increasing number of bits, since the tree is sorted, the node with the lowest count in the depth will be demoted
            let demote_node_pos = depth_index[check_depth as usize].start;
            tree.nodes[demote_node_pos as usize].number_bits += 1;
            // update move depth_index
            depth_index[check_depth as usize].start += 1;
            depth_index[check_depth as usize + 1].end += 1;
            // repay debt
            let repay = 1 << max_bits - check_depth - 1;
            // to allow multiple demotes of one symbol, we need to go back one level again
            if check_depth != max_bits - 1 {
                check_depth += 1;
            }
            debt -= repay;
        }
    }
    if debt > 0 {
        panic!("could not repay debt, this should not happen");
    }

    if debt < 0 {
        // if we overshoot and pay back too much debt, the tree may be in an invalid state, which can cause it to generate wrong prefix codes
        let mut check_depth = max_bits;
        while debt<0 {
            if depth_index[check_depth as usize].is_empty() {
                check_depth -= 1;
                if check_depth == 0{
                    panic!("could not repay debt");
                }
                continue;
            }
            let promote_node_pos = depth_index[check_depth as usize].end - 1; // get last node -1, because not inclusive
            tree.nodes[promote_node_pos as usize].number_bits -= 1;

            depth_index[check_depth as usize].end -= 1;
            depth_index[check_depth as usize - 1].start -= 1;

            debt += 1;
        }
    }

}

type DepthIndex = [RangeExlusive; 16];

#[derive(Debug, Default, Clone, Copy)]
struct RangeExlusive {
    start: u16,
    end: u16,
}

impl RangeExlusive {
    fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Will build an index with start end position of each depth level of the tree.
/// max tree height supported is 16, so it should be normalized by set_max_height before.
/// Nodes must be sorted by number_bits (depth) DESC, which is the case for `build_tree_fast_1`.
#[inline]
fn get_depth_index(nodes: &[Node]) -> DepthIndex {
    let mut depth_index = [RangeExlusive::default(); 16];

    depth_index[nodes[0].number_bits as usize].start = 0;
    let mut current_num_bits = nodes[0].number_bits;
    let mut pos = 0;
    while nodes[pos].number_bits > 0 {
        if nodes[pos].number_bits != current_num_bits {
            // fill empty depths with closest value (for easier movement of the borders later)
            for num_bits in nodes[pos].number_bits..current_num_bits {
                depth_index[num_bits as usize].start = pos as u16;
                depth_index[num_bits as usize].end = pos as u16;
            }
            depth_index[current_num_bits as usize].end = pos as u16;
            current_num_bits = nodes[pos].number_bits;
            depth_index[current_num_bits as usize].start = pos as u16;
        }

        pos += 1;
    }
    depth_index[current_num_bits as usize].end = pos as u16;

    depth_index
}

#[inline]
pub fn increment_return_old(val: &mut usize) -> usize {
    *val += 1;
    *val - 1
}

#[inline]
pub fn decrement_return_old(val: &mut usize) -> usize {
    *val -= 1;
    *val + 1
}

/// will validate the table to have generated correct prefix properties for all symbols.
/// This validation is rather slow and should be used in a regular compression execution.
pub fn test_prefix_property(table: &[MinNode; 256])  {
    let mut node_by_num_bits: Vec<Vec<MinNode>> = vec![];
    node_by_num_bits.resize(16, vec![]);
    let mut max_bits = 0;
    for el in table {
        node_by_num_bits[el.number_bits as usize].push(*el);
        max_bits = max_bits.max(el.number_bits);
    }
    for num_bits in (2..=max_bits).rev() {
        // let mut lower_bits_nodes = vec![];
        let lower_bits_nodes: Vec<MinNode> = (1..num_bits-1).flat_map(|num_bits|node_by_num_bits[num_bits as usize].to_vec()).collect();
        for node in &node_by_num_bits[num_bits as usize] {
            // find any node in the lower bits which has the same prefix
            for comp_node in &lower_bits_nodes{
                let bit_diff = num_bits - comp_node.number_bits;
                if (node.val >> bit_diff) == comp_node.val {
                    panic!("invalid prefix detected between {:?} and {:?}", node, comp_node);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::count_simple;

    fn get_test_nodes_depth_5() -> [Node; 512] {
        let mut nodes = [Node::default(); 512];
        nodes[0] = Node {
            symbol: Some(1),
            count: 9,
            number_bits: 5,
            ..Default::default()
        };
        nodes[1] = Node {
            symbol: Some(2),
            count: 10,
            number_bits: 5,
            ..Default::default()
        };
        nodes[2] = Node {
            symbol: Some(3),
            count: 50,
            number_bits: 4,
            ..Default::default()
        };
        nodes[3] = Node {
            symbol: Some(4),
            count: 100,
            number_bits: 4,
            ..Default::default()
        };
        nodes[4] = Node {
            symbol: Some(5),
            count: 100,
            number_bits: 4,
            ..Default::default()
        };
        nodes[5] = Node {
            symbol: Some(6),
            count: 100,
            number_bits: 2,
            ..Default::default()
        };
        nodes[6] = Node {
            symbol: Some(7),
            count: 100,
            number_bits: 1,
            ..Default::default()
        };
        nodes
    }

    #[test]
    fn test_get_depth_index() {
        let nodes = get_test_nodes_depth_5();
        let index = get_depth_index(&nodes);
        assert_eq!(index[1].start, 6);
        assert_eq!(index[1].end, 7);
        assert_eq!(index[2].start, 5);
        assert_eq!(index[2].end, 6);
        assert_eq!(index[3].start, 5);
        assert_eq!(index[3].end, 5);
        assert_eq!(index[4].start, 2);
        assert_eq!(index[4].end, 5);
        assert_eq!(index[5].start, 0);
        assert_eq!(index[5].end, 2);
        assert_eq!(index[3].is_empty(), true);
        assert_eq!(index[5].is_empty(), false);
    }

    #[test]
    fn test_max_height_1() {
        let nodes = get_test_nodes_depth_5();
        let mut tree = Tree {
            nodes: nodes.to_vec(),
            root_node: 99, // unused
            last_symbol_node_pos: 6,
        };

        set_max_height(&mut tree, 3);

        assert_eq!(tree.nodes.iter().map(|n| n.number_bits).max().unwrap(), 3);
    }

    #[test]
    fn test_max_height_check_limits() {
        // only 4 nodes and height of 2 means the tree will be converted to a perfectly balanced tree 
        // doesn't make sense for compression, but to verify the check limits of the algorithm
        let src: Vec<u8> = vec![
            1, 2, 3, 3, 4, 4, 4, 4, 4, 4, 4,
        ];

        let counts = count_simple(&src);
        let mut tree = build_tree_fast_1(&counts);

        let max_height = 2;
        set_max_height(&mut tree, max_height);

        assert_eq!(tree.nodes.iter().map(|n| n.number_bits).max().unwrap(), 2);

        test_prefix_property(&tree_to_table(&tree));
    }

    #[test]
    #[should_panic(expected = "minimum_tree_depth")]
    fn test_max_height_too_small() {
        // 5 nodes and height of 2 means the tree will too small
        let src: Vec<u8> = vec![
            1, 2, 3, 3, 4, 4, 4, 4, 4, 4, 4, 5
        ];

        let counts = count_simple(&src);
        let mut tree = build_tree_fast_1(&counts);

        let max_height = 2;
        set_max_height(&mut tree, max_height);

        assert_eq!(tree.nodes.iter().map(|n| n.number_bits).max().unwrap(), 2);
        test_prefix_property(&tree_to_table(&tree));
    }

    #[test]
    fn test_max_height_2_special_case() {
        // in this case the first level to try to repay debt is empty, the next level will pay too much
        let src: Vec<u8> = vec![
            1, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        ];

        let counts = count_simple(&src);
        let mut tree = build_tree_fast_1(&counts);

        let max_height = 3;
        set_max_height(&mut tree, max_height);
        test_prefix_property(&tree_to_table(&tree));
    }

    #[test]
    fn issue_1_max_height_debt_repay() {
        // limit assertions were wrong
        let src: Vec<u8> = vec![
            1, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        ];

        let counts = count_simple(&src);
        let mut tree = build_tree_fast_1(&counts);
        // println!("{}", tree);

        let max_height = 4;
        set_max_height(&mut tree, max_height);
        test_prefix_property(&tree_to_table(&tree));
    }

    #[test]
    fn test_biggest_count_symbol() {
        let src: Vec<u8> = vec![
            1, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7,
        ];

        let counts = count_simple(&src);
        let tree = build_tree_fast_1(&counts);
        assert_eq!(tree.nodes[tree.last_symbol_node_pos as usize].symbol, Some(6) );
        test_prefix_property(&tree_to_table(&tree));
    }


    #[test]
    fn max_height_check_count_order() {
        let src: Vec<u8> = vec![
            1, 2, 3, 3, 4, 4, 5, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 8
        ];

        let counts = count_simple(&src);
        let mut tree = build_tree_fast_1(&counts);
        assert_eq!(tree.nodes[3].symbol, Some(3));
        assert_eq!(tree.nodes[3].number_bits, 3);
        // println!("{}", tree);

        let max_height = 4;
        set_max_height(&mut tree, max_height);

        assert_eq!(tree.nodes[3].symbol, Some(3));
        assert_eq!(tree.nodes[3].number_bits, 4);
        test_prefix_property(&tree_to_table(&tree));
    }
    #[test]
    fn set_max_height_multiple_levels() {
        let src: Vec<u8> = vec![
            1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,2,2,2,2,2,2,2,2,2,2,3,3,3,3,3,3,4,4,4,5,6
        ];
        let counts = count_simple(&src);
        let mut tree = build_tree_fast(&counts);
        // tree has height of 5
        assert_eq!(tree.nodes[0].number_bits, 5);
        // println!("{}", tree);

        let max_height = 3;
        set_max_height(&mut tree, max_height);
        let _table = tree_to_table(&tree);
        test_prefix_property(&tree_to_table(&tree));
        // dbg!(&table[..7]);
        // assert_eq!(tree.nodes[3].symbol, Some(3));
        // assert_eq!(tree.nodes[3].number_bits, 3);
    }
    #[test]
    fn fuzzer_issue_1_255_value() {
        let src: Vec<u8> = vec![
            255
        ];
        let counts = count_simple(&src);
        let _tree = build_tree_fast(&counts);
    }
    #[test]
    fn fuzzer_issue_2_repay_debt() {
        let src: Vec<u8> = vec![
            183, 47, 40, 107, 107, 93, 107, 107, 107, 107, 107, 107, 107, 107, 107, 107, 104, 58, 43
        ];
        let counts = count_simple(&src);
        let mut tree = build_tree_fast(&counts);
        println!("{}", tree);

        let min_tree_depth = minimum_tree_depth(tree.get_num_symbol_nodes() as usize);
        dbg!(min_tree_depth);
        dbg!(tree.get_depth());
        set_max_height(&mut tree, 3);
        // if tree.get_depth() as usize - 1 >= min_tree_depth{
    }

}


// huffNode[0] count:21 byte:1 
// huffNode[1] count:10 byte:2 
// huffNode[2] count:6 byte:3 
// huffNode[3] count:3 byte:4 
// huffNode[4] count:1 byte:5 
// huffNode[5] count:1 byte:6 
// huffNode[6] count:0 byte:0 