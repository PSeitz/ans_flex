
use bitstream::BitDStreamReverse;
use crate::HUF_TABLELOG_MAX;
use bitstream::NUM_BITS_IN_BIT_CONTAINER;
use bitstream::BitCstream;
use crate::tree::tree_node::MinNode;

/// compress input to dst in reverse order
fn compress_1x_rev(table: &[MinNode], input: &[u8], dst: &mut[u8]) {
    let mut bit_c = BitCstream::new();
    
    let mut index = input.len();

    let mod_4 = input.len() & 3;
    if mod_4 >= 3 {
        index -=1;
        huf_encode_symbol(input[index], &mut bit_c, &table);
        huf_flush_bits_2(&mut bit_c, dst);
    }
    if mod_4 >= 2 {
        index -=1;
        huf_encode_symbol(input[index], &mut bit_c, &table);
        huf_flush_bits_1(&mut bit_c, dst);
    }
    if mod_4 >= 1 {
        index -=1;
        huf_encode_symbol(input[index], &mut bit_c, &table);
        bit_c.flush_bits_fast(dst);
    }

    for chunk in input[..index].rchunks_exact(4) {
        huf_encode_symbol(chunk[3], &mut bit_c, &table);
        huf_encode_symbol(chunk[2], &mut bit_c, &table);
        huf_encode_symbol(chunk[1], &mut bit_c, &table);
        huf_encode_symbol(chunk[0], &mut bit_c, &table);
        bit_c.flush_bits_fast(dst);
    }
    bit_c.finish_stream(dst);

}


fn huf_encode_symbol(symbol: u8, bit_c: &mut BitCstream, table: &[MinNode]) {
    let node = table[symbol as usize];
    bit_c.add_bits_fast(node.val as usize, node.number_bits as u32);
}


// fn huf_decode_symbol(symbol: u8, bit_c: &mut BitDStreamReverse, table: &[MinNode]) {
//     let node = table[symbol as usize];
//     bit_c.add_bits_fast(node.val as usize, node.number_bits as u32);
// }


fn huf_flush_bits_1(bit_c: &mut BitCstream, dst: &mut[u8]) {
    if NUM_BITS_IN_BIT_CONTAINER > HUF_TABLELOG_MAX * 2 + 7 {
        bit_c.flush_bits_fast(dst);
    }
}

fn huf_flush_bits_2(bit_c: &mut BitCstream, dst: &mut[u8]) {
    if NUM_BITS_IN_BIT_CONTAINER > HUF_TABLELOG_MAX * 4 + 7 {
        bit_c.flush_bits_fast(dst);
    }
}

#[cfg(test)]
mod tests {
    use crate::compress::compress_1x_rev;
    use crate::tree::build_tree::tree_to_table;
    use crate::build_tree_fast;
    use crate::count_simple;

    #[test]
    fn test_compress() {
        const TEST_DATA: &'static [u8] = include_bytes!("../../test_data/compression_65k.txt");
        let counts = count_simple(&TEST_DATA);
        let tree = build_tree_fast(&counts);
        println!("estimate_compressed_size: {:?}", tree.estimate_compressed_size());
        println!("ratio: {:?}", tree.estimate_compressed_size() as f32 / TEST_DATA.len() as f32);

        let table = tree_to_table(&tree);
        let mut out = vec![];
        out.resize(tree.estimate_compressed_size() + 16, 0);
        compress_1x_rev(&table, TEST_DATA, &mut out);
    }

    #[test]
    fn test_compress_simple() {

        const TEST_DATA: &'static [u8] = &[1,1,2,3, 1, 1];
        let counts = count_simple(&TEST_DATA);
        let tree = build_tree_fast(&counts);
        println!("estimate_compressed_size: {:?}", tree.estimate_compressed_size());
        println!("ratio: {:?}", tree.estimate_compressed_size() as f32 / TEST_DATA.len() as f32);

        let table = tree_to_table(&tree);
        let mut out = vec![];
        out.resize(tree.estimate_compressed_size() + 16, 0);
        compress_1x_rev(&table, TEST_DATA, &mut out);
        println!("{:08b}", out[0]);
        println!("{:08b}", out[1]);
        println!("{:08b}", out[2]);
    }
}