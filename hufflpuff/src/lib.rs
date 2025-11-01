pub mod compress;
pub mod decompress;
pub mod tree;
use crate::tree::tree_node::Node;
use crate::tree::Tree;
use std::collections::BinaryHeap;

pub use crate::tree::build_tree_fast;

pub const MAX_SYMBOL_VALUE: u32 = u8::MAX as u32;

pub const HUF_CTABLEBOUND: u32 = 129;

pub const HUF_CTABLE_WORKSPACE_SIZE: u32 = 2 * MAX_SYMBOL_VALUE + 1 + 1;
pub const HUF_BLOCKSIZE_MAX: u32 = 128 * 1024;
pub const HUF_WORKSPACE_SIZE: u32 = (6 << 10) + 256;

pub const HUF_TABLELOG_MAX: u32 = 12;

/// Absolute MAX, beyond that code doesn't work
pub const HUF_TABLELOG_ABSOLUTEMAX: u32 = 15;
pub const HUF_TABLELOG_DEFAULT: u32 = 11;

pub fn huf_blockbound(size: usize) -> usize {
    size + (size >> 8) + 8
}
pub fn huf_compressbound(size: usize) -> usize {
    huf_blockbound(size) + HUF_CTABLEBOUND as usize
}

// // return bits written
// fn encode_huff(state: &mut u32, symbol: u8, codes: &HuffmanCodes) -> usize {
//     *state <<= codes.code_length[symbol as usize];
//     *state |= ((codes.code[symbol as usize]))  as u32;
//     codes.code_length[symbol as usize] as usize
// }

// fn encode(state: &mut u32, val: u32, modulo: u32) {
//     *state = *state * modulo + val;
// }

// fn decode(state: &mut u32, modulo: u32) -> u32 {
//     let val = *state % modulo;
//     *state /= modulo;
//     return val;
// }

#[derive(Debug)]
#[allow(dead_code)]
pub struct HuffmanCodes {
    /// symbol to code length
    code_length: [u8; 8],
    /// symbol to code
    code: [u8; 8],
}

/// creates a table with the counts of each symbol
pub fn build_lookup_tables_from_tree(tree: &mut Tree) -> HuffmanCodes {
    let mut code_length = [0_u8; 8];
    let mut code = [0_u8; 8];

    tree.walk_tree(tree.root_node, &mut |node, transitions, depth| {
        if let Some(symbol) = node.symbol {
            code_length[symbol as usize] = depth as u8;
            code[symbol as usize] = transitions as u8;
        }
    });

    dbg!(code_length);
    dbg!(code);
    HuffmanCodes { code_length, code }
}

/// creates a table with the counts of each symbol
/// very simple and slow method to create a huffman tree
#[inline]
pub fn build_tree_heap(counts: &[usize; 256]) -> Vec<Node> {
    let mut heap = BinaryHeap::with_capacity(256);
    for (byte, count) in counts.iter().enumerate() {
        if *count != 0 {
            heap.push(Node {
                symbol: Some(byte as u8),
                count: *count as u32,
                ..Default::default()
            });
        }
    }

    let mut nodes = vec![];
    while let (Some(el1), el2) = (heap.pop(), heap.pop()) {
        if let Some(el2) = el2 {
            nodes.push(el1);
            nodes.push(el2);
            // add internal Node with aggregated count
            heap.push(Node {
                count: el1.count + el2.count,
                left: Some(nodes.len() as u16 - 2),
                right: Some(nodes.len() as u16 - 1),
                ..Default::default()
            });
        } else {
            // last node, which will be the root node
            nodes.push(el1);
        }
    }
    // tree.root_node = tree.nodes.len() - 1;

    // dbg!(tree.get_root_node());
    // dbg!(&tree);
    // tree

    nodes
}

/// creates a table with the counts of each symbol
#[inline]
pub fn count_simple(input: &[u8]) -> [usize; 256] {
    let mut counts = [0; 256];

    for byte in input {
        counts[*byte as usize] += 1
    }
    counts
}

#[inline]
pub fn get_max_value(counts: &[usize; 256]) -> usize {
    *counts.iter().max().unwrap()
}

#[cfg(test)]
mod tests {

    use crate::compress::compress_1x_rev;
    use crate::tree::build_tree::test_prefix_property;
    use crate::tree::build_tree::tree_to_table;
    use crate::tree::build_tree_fast;
    use crate::*;
    use std::collections::HashSet;

    #[test]
    fn test_example() {
        let all_bytes: Vec<u8> = vec![
            // 0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,1,1,1,2,2,2,4,5,6
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 4, 5,
        ];

        use std::io::Write;
        std::fs::File::create("../../FiniteStateEntropy/programs/test_data_100")
            .unwrap()
            .write_all(&all_bytes)
            .unwrap();

        let mut test_data = vec![];
        use std::io::Read;
        std::fs::File::open("../../FiniteStateEntropy/programs/test_data_100")
            .unwrap()
            .read_to_end(&mut test_data)
            .unwrap();

        let counts = count_simple(&test_data);
        let tree = build_tree_fast(&counts);

        let table = tree_to_table(&tree);
        let mut out = vec![0; tree.estimate_compressed_size() + 16];
        compress_1x_rev(&table, &test_data, &mut out);

        // compress_1x_rev(table: &[MinNode], input: &[u8], dst: &mut [u8]) {
        println!("{}", tree);
    }
    #[test]
    fn special_case() {
        let src: Vec<u8> = vec![
            1, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        ];

        let counts = count_simple(&src);
        let tree = build_tree_fast(&counts);

        // println!("{}", tree);
        test_prefix_property(&tree_to_table(&tree));
    }
    #[test]
    fn simple_balanced() {
        let src: Vec<u8> = vec![1, 2, 3, 4];

        let counts = count_simple(&src);
        let tree = build_tree_fast(&counts);
        test_prefix_property(&tree_to_table(&tree));
    }

    #[test]
    fn balanced_tree_distribution() {
        let all_bytes = (0..=u8::MAX).collect::<Vec<u8>>();
        let counts = count_simple(&all_bytes);
        let tree = build_tree_fast(&counts);
        test_prefix_property(&tree_to_table(&tree));
    }

    // input is the number of repeats per symbol
    fn gen_fibo_distribution(fibo_counts: &[u64]) -> Vec<u8> {
        use std::io::Read;
        let mut all_bytes = Vec::new();

        for (num, repeat) in fibo_counts.iter().enumerate() {
            std::io::repeat(num as u8)
                .take(*repeat)
                .read_to_end(&mut all_bytes)
                .unwrap();
        }
        all_bytes
    }

    fn test_fibonacci(fibo_counts: &[u64]) {
        let all_bytes = gen_fibo_distribution(fibo_counts);

        let counts = count_simple(&all_bytes);
        let tree = build_tree_fast(&counts);

        assert!(tree.nodes[tree.root_node].left.is_some());
        assert!(tree.nodes[tree.root_node].right.is_some());

        let root_left = tree.nodes[tree.root_node].left.unwrap();
        // in the fibonacci case the biggest symbol count should be directly under the root node
        assert!(tree.nodes[root_left as usize].symbol.is_some());

        validate_tree(&tree);
    }

    #[test]
    fn long_tree_distribution() {
        test_fibonacci(&[1_u64, 1, 2, 3]);
        test_fibonacci(&[1_u64, 1, 2, 3, 5]);
        test_fibonacci(&[1_u64, 1, 2, 3, 5, 8]);
        test_fibonacci(&[1_u64, 1, 2, 3, 5, 8, 13]);
        test_fibonacci(&[1_u64, 1, 2, 3, 5, 8, 13, 21]);
    }

    fn validate_tree(tree: &Tree) {
        // check all nodes are connected
        let mut all_nodes = tree
            .nodes
            .iter()
            .filter(|n| n.count != 0)
            .cloned()
            .collect::<HashSet<Node>>();
        all_nodes.remove(tree.get_root_node());
        tree.walk_tree(tree.root_node, &mut |node, _transitions, _depth| {
            all_nodes.remove(node);
        });
        assert_eq!(all_nodes.len(), 0);

        // check count of childs are always lower
        tree.walk_tree(tree.root_node, &mut |node, _transitions, _depth| {
            if let Some(left) = node.left {
                let child_left = tree.get_node(left);
                assert!(child_left.count < node.count);
            }
        });
        test_prefix_property(&tree_to_table(tree));
    }

    // #[test]
    // fn test_data_test() {
    //     let test_datas = [
    //         include_bytes!("../../test_data/compression_66k_JSON.txt") as &'static [u8],
    //         include_bytes!("../../test_data/compression_65k.txt") as &'static [u8],
    //         include_bytes!("../../test_data/compression_34k.txt") as &'static [u8],
    //         include_bytes!("../../test_data/compression_1k.txt") as &'static [u8],
    //         include_bytes!("../../test_data/v4_uuids_19k.txt") as &'static [u8],
    //         include_bytes!("../../test_data/v4_uuids_93k.txt") as &'static [u8],
    //     ];
    //     for test_data in &test_datas {
    //         let counts = count_simple(&test_data);
    //         let tree = build_tree_fast(&counts);
    //         validate_tree(&tree);
    //     }
    // }
    #[test]
    fn test_66k_json() {
        const TEST_DATA: &[u8] = include_bytes!("../../test_data/compression_66k_JSON.txt");
        let tree = test_tree(TEST_DATA);
        println!(
            "estimate_compressed_size: {:?}",
            tree.estimate_compressed_size()
        );
        println!(
            "ratio: {:?}",
            tree.estimate_compressed_size() as f32 / TEST_DATA.len() as f32
        );
    }
    #[test]
    fn test_65k_text() {
        const TEST_DATA: &[u8] = include_bytes!("../../test_data/compression_65k.txt");
        let counts = count_simple(TEST_DATA);
        let tree = build_tree_fast(&counts);
        validate_tree(&tree);
        println!(
            "estimate_compressed_size: {:?}",
            tree.estimate_compressed_size()
        );
        println!(
            "ratio: {:?}",
            tree.estimate_compressed_size() as f32 / TEST_DATA.len() as f32
        );
    }
    #[test]
    fn test_34k_text() {
        const TEST_DATA: &[u8] = include_bytes!("../../test_data/compression_34k.txt");
        test_tree(TEST_DATA);
    }
    #[test]
    fn test_1k_text() {
        const TEST_DATA: &[u8] = include_bytes!("../../test_data/compression_1k.txt");
        test_tree(TEST_DATA);
    }

    #[test]
    fn test_v4_uuids_19_k() {
        const TEST_DATA: &[u8] = include_bytes!("../../test_data/v4_uuids_19k.txt");
        test_tree(TEST_DATA);
    }
    #[test]
    fn test_v4_uuids_93_k() {
        const TEST_DATA: &[u8] = include_bytes!("../../test_data/v4_uuids_93k.txt");
        test_tree(TEST_DATA);
    }

    fn test_tree(data: &[u8]) -> tree::Tree {
        let counts = count_simple(data);
        let tree = build_tree_fast(&counts);
        validate_tree(&tree);
        tree
    }

    #[test]
    fn test_prefix_codes_zstd_format_example() {
        let src: Vec<u8> = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 4, 5,
        ];

        let counts = count_simple(&src);
        let tree = build_tree_fast(&counts);

        let table = tree_to_table(&tree);
        test_prefix_property(&tree_to_table(&tree));

        // check prefix codes
        assert_eq!(table[0].val, 1); // 1
        assert_eq!(table[0].number_bits, 1);
        assert_eq!(table[1].val, 1); // 01
        assert_eq!(table[1].number_bits, 2);
        assert_eq!(table[2].val, 1); // 001
        assert_eq!(table[2].number_bits, 3);
        assert_eq!(table[3].val, 0);
        assert_eq!(table[3].number_bits, 0);
        assert_eq!(table[4].val, 0); // 0000
        assert_eq!(table[4].number_bits, 4);
        assert_eq!(table[5].val, 1); // 0001
        assert_eq!(table[5].number_bits, 4);
    }
}
