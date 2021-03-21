#![no_main]

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
    let tree = build_tree_fast(&counts);
    test_prefix_property(&tree_to_table(&tree));
});
