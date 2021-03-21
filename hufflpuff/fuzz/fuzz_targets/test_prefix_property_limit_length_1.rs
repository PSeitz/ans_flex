#![no_main]

use hufflpuff::tree::build_tree::set_max_height;
use hufflpuff::tree::minimum_tree_depth;
use hufflpuff::count_simple;
use hufflpuff::build_tree_fast;
use hufflpuff::tree::build_tree::test_prefix_property;
use hufflpuff::tree::build_tree::tree_to_table;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() == 0 {
        return;
    }
    let counts = count_simple(&data);
    let mut tree = build_tree_fast(&counts);
    let min_tree_depth = minimum_tree_depth(tree.get_num_symbol_nodes() as usize);
    if tree.get_depth() as usize - 1 >= min_tree_depth{
        let new_depth = tree.get_depth() - 1;
        set_max_height(&mut tree, new_depth);
        test_prefix_property(&tree_to_table(&tree));
    }
});
