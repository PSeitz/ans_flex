use crate::bitstream::{BitDstream, BitDstreamStatus};
use crate::table::DecompressionTable;

/// Decomprssion State context. Multiple ones are possible
#[derive(Debug)]
struct FseDState {
    state: usize,
}

impl FseDState {
    fn new(bit_stream: &mut BitDstream, table_log: u32, input: &[u8]) -> Self {
        let state = bit_stream.read_bits(table_log as u32);
        bit_stream.reload_stream(input);
        // DStatePtr->table = dt + 1;  TODO?
        FseDState { state }
    }
}

/// decompressed into output
///
/// output needs to be preallocated to fit the uncompressed output
///
/// 
#[inline]
pub fn fse_decompress(output: &mut Vec<u8>, input: &[u8], table: &DecompressionTable, table_log: u32) {
    let mut bit_stream = BitDstream::new(input);

    let mut state1 = FseDState::new(&mut bit_stream, table_log, input);
    let mut state2 = FseDState::new(&mut bit_stream, table_log, input);

    // 64bit version
    // let out_len = output.len();
    // let mut iter = output[..out_len.saturating_sub(80)].chunks_exact_mut(4);
    let mut iter = output.chunks_exact_mut(4);
    let mut consumed = 0;
    for out_chunk in &mut iter {
        let status = bit_stream.reload_stream(input);
        // let status = bit_stream.reload_stream_fast(input);
        if  status != BitDstreamStatus::Unfinished  {
            // panic!("consumed {:?} unconsumed {:?}", consumed, output.len() - consumed);
            break;
        }
        out_chunk[0] = fse_decode_symbol(table, &mut state1, &mut bit_stream, table.fast);
        out_chunk[1] = fse_decode_symbol(table, &mut state2, &mut bit_stream, table.fast);
        out_chunk[2] = fse_decode_symbol(table, &mut state1, &mut bit_stream, table.fast);
        out_chunk[3] = fse_decode_symbol(table, &mut state2, &mut bit_stream, table.fast);
        // consumed += 4;
        // let status = bit_stream.reload_stream(input);
        // if  status != BitDstreamStatus::Unfinished  {
        //     break;
        // }
        // out_chunk[4] = fse_decode_symbol(table, &mut state1, &mut bit_stream, table.fast);
        // out_chunk[5] = fse_decode_symbol(table, &mut state2, &mut bit_stream, table.fast);
        // out_chunk[6] = fse_decode_symbol(table, &mut state1, &mut bit_stream, table.fast);
        // out_chunk[7] = fse_decode_symbol(table, &mut state2, &mut bit_stream, table.fast);
        consumed += 4;
    }

    #[cfg(target_pointer_width = "32")]
    {
        panic!("32bit decompression not yet implemented");
    }
    // let remainder_chunk = iter.into_remainder();
    let remainder_chunk = &mut output[consumed..];
    let mut remainder_pos = 0;
    loop {
        remainder_chunk[remainder_pos] = fse_decode_symbol(table, &mut state1, &mut bit_stream, table.fast);
        remainder_pos+=1;
        if bit_stream.reload_stream(input) == BitDstreamStatus::Overflow  {
            remainder_chunk[remainder_pos] = fse_decode_symbol(table, &mut state2, &mut bit_stream, table.fast);
            break;
        }

        remainder_chunk[remainder_pos] = fse_decode_symbol(table, &mut state2, &mut bit_stream, table.fast);
        remainder_pos+=1;
        if bit_stream.reload_stream(input) == BitDstreamStatus::Overflow  {
            remainder_chunk[remainder_pos] = fse_decode_symbol(table, &mut state1, &mut bit_stream, table.fast);
            break;
        }

    }

}

#[inline]
fn fse_decode_symbol(
    table: &DecompressionTable,
    d_state: &mut FseDState,
    bit_d: &mut BitDstream,
    fast:bool
) -> u8 {
    if fast {
        internal_fse_decode_symbol_fast(table, d_state, bit_d)
    }else {
        internal_fse_decode_symbol(table, d_state, bit_d)
    }
}

/// unsafe, only works if no symbol has a probability > 50%
#[inline]
fn internal_fse_decode_symbol_fast(
    table: &DecompressionTable,
    d_state: &mut FseDState,
    bit_d: &mut BitDstream,
) -> u8 {
    let d_info = unsafe{table.table.get_unchecked(d_state.state)};

    let low_bits = bit_d.read_bits_fast(d_info.nb_bits as u32);

    // println!("oldstate {:?} d_info.new_state {:?} low_bits {:?} --> new state {:?}", d_state.state, d_info.new_state, low_bits, d_info.new_state as usize + low_bits);
    d_state.state = d_info.new_state as usize + low_bits;

    return d_info.symbol;
}

#[inline]
fn internal_fse_decode_symbol(
    table: &DecompressionTable,
    d_state: &mut FseDState,
    bit_d: &mut BitDstream,
) -> u8 {
    let d_info = unsafe{table.table.get_unchecked(d_state.state)};
    // let d_info = table.table[d_state.state];

    let low_bits = bit_d.read_bits(d_info.nb_bits as u32);
    // println!("oldstate {:?} d_info.new_state {:?} low_bits {:?} : symbol {:?}  --> new state {:?}", d_state.state, d_info.new_state, low_bits, d_info.symbol, d_info.new_state as usize + low_bits);
    d_state.state = d_info.new_state as usize + low_bits;
    return d_info.symbol;
}

