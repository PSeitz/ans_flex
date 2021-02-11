use crate::bitstream::BitDstream;
use crate::table::CompressionTable;
use crate::table::DecompressionTable;

#[inline]
pub fn fse_decompress(input: &[u8], comp: &CompressionTable, table_log: u32) -> Vec<u8> {
    vec![]
}

/// unsafe, only works if no symbol has a probability > 50%
fn fse_decode_symbol_fast(
    table: &DecompressionTable,
    d_state: &mut FseDecompressionState,
    bit_d: &mut BitDstream,
) -> u8 {
    let d_info = table[d_state.state];

    let low_bits = bit_d.read_bits_fast(d_info.nbBits as u32);
    d_state.state += low_bits;
    return d_info.symbol;
}

fn fse_decode_symbol(
    table: &DecompressionTable,
    d_state: &mut FseDecompressionState,
    bit_d: &mut BitDstream,
) -> u8 {
    let d_info = table[d_state.state];

    let low_bits = bit_d.read_bits(d_info.nbBits as u32);
    d_state.state += low_bits;
    return d_info.symbol;
}

/// Decomprssion State context. Multiple ones are possible
#[derive(Debug)]
struct FseDecompressionState {
    state: usize,
}
