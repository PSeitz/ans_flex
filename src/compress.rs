use crate::table::CompressionTable;
use crate::FSE_MAX_TABLELOG;
use bitstream::BitCstream;
use bitstream::BitCstreamOwned;
use bitstream::BIT_CONTAINER_BYTES;
use bitstream::NUM_BITS_IN_BIT_CONTAINER;

#[derive(Debug)]
struct FseCState {
    value: usize,
}

impl FseCState {
    fn new(symbol: u8, comp: &CompressionTable) -> Self {
        let symbol_tt = comp.symbol_tt[symbol as usize];
        let nb_bits_out: u32 = ((symbol_tt.delta_nb_bits as usize + (1 << 15)) >> 16) as u32;
        let value: usize = ((nb_bits_out as usize) << 16) - symbol_tt.delta_nb_bits as usize;
        let value: usize = comp.state_table
            [((value >> nb_bits_out) as isize + symbol_tt.delta_find_state as isize) as usize]
            as usize;

        // println!("NEW symbol {:?} nb_bits_out {:?} c_state.value {:?}", symbol, nb_bits_out, value);

        FseCState { value }
    }
}

// FSE buffer bounds

/// Maximum size to store counts
pub const FSE_NCOUNTBOUND: usize = 512;

#[inline]
fn fse_blockbound(size: usize) -> usize {
    size + (size>>7) + 4 /* fse states */ + BIT_CONTAINER_BYTES
}

#[inline]
fn fse_compressbound(size: usize) -> usize {
    FSE_NCOUNTBOUND + fse_blockbound(size)
}

#[inline]
pub fn fse_compress<'a>(input: &[u8], comp: &CompressionTable, table_log: u32) -> BitCstreamOwned {
    assert!(input.len() > 2);
    let max_compressed_size = fse_compressbound(input.len());

    let mut data: Vec<u8> = Vec::new();
    data.resize(max_compressed_size, 0);
    let mut bit_c = BitCstream::new();

    // let mut bit_c = BitCstream::new(max_compressed_size);

    let (stream_data_pos, stream_bit_pos) = {
        let mut index = input.len();

        let (mut state1, mut state2) = if input.len() & 1 == 1 {
            index -= 1;
            let mut state1 = FseCState::new(input[index], comp);
            index -= 1;
            let state2 = FseCState::new(input[index], comp);
            index -= 1;
            fse_encode_symbol(&mut bit_c, &mut state1, comp, input[index]);
            bit_c.flush_bits_fast(&mut data);
            (state1, state2)
        } else {
            index -= 1;
            let state2 = FseCState::new(input[index], comp);
            index -= 1;
            let state1 = FseCState::new(input[index], comp);
            (state1, state2)
        };

        // join to mod 4
        if NUM_BITS_IN_BIT_CONTAINER > FSE_MAX_TABLELOG * 4 + 7
            // test bit 2
            && ((input.len() - 2) & 2) == 2
        {
            index -= 1;
            fse_encode_symbol(&mut bit_c, &mut state2, comp, input[index]);
            index -= 1;
            fse_encode_symbol(&mut bit_c, &mut state1, comp, input[index]);
            bit_c.flush_bits_fast(&mut data);
        }
        // println!("START LOOP");

        // these loops are correct for FSE_MAX_TABLELOG = 12
        #[cfg(target_pointer_width = "64")]
        {
            // 64 bit version
            for chunk in input[..index].rchunks_exact(4) {
                fse_encode_symbol(&mut bit_c, &mut state2, comp, chunk[3]);
                fse_encode_symbol(&mut bit_c, &mut state1, comp, chunk[2]);
                fse_encode_symbol(&mut bit_c, &mut state2, comp, chunk[1]);
                fse_encode_symbol(&mut bit_c, &mut state1, comp, chunk[0]);
                bit_c.flush_bits_fast(&mut data);
            }
        }

        #[cfg(target_pointer_width = "32")]
        {
            // 32 bit version
            for chunk in input[..index].rchunks_exact(2) {
                fse_encode_symbol(&mut bit_c, &mut state2, comp, chunk[1]);
                bit_c.flush_bits_fast(&mut data);
                fse_encode_symbol(&mut bit_c, &mut state1, comp, chunk[0]);
                bit_c.flush_bits_fast(&mut data);
            }
        }

        fse_flush_cstate(&mut bit_c, &mut state2, table_log, &mut data);
        fse_flush_cstate(&mut bit_c, &mut state1, table_log, &mut data);

        bit_c.finish_stream(&mut data);
        (bit_c.data_pos, bit_c.bit_pos)
    };

    BitCstreamOwned::new(data, stream_data_pos, stream_bit_pos)
}

#[inline]
fn fse_encode_symbol(
    bit_c: &mut BitCstream,
    c_state: &mut FseCState,
    comp: &CompressionTable,
    symbol: u8,
) {
    unsafe {
        // These unchecked access bring aroung 3-14% gain
        let symbol_tt = comp.symbol_tt.get_unchecked(symbol as usize);
        // let symbol_tt = comp.symbol_tt[symbol as usize];

        let nb_bits_out: u32 = ((c_state.value + symbol_tt.delta_nb_bits as usize) >> 16) as u32;
        bit_c.add_bits(c_state.value, nb_bits_out);
        let state_index = ((c_state.value >> nb_bits_out) as isize
            + symbol_tt.delta_find_state as isize) as usize;

        // c_state.value = comp.state_table [state_index] as usize;
        c_state.value = *comp.state_table.get_unchecked(state_index) as usize;
        // println!("symbol {:?} nb_bits_out {:?} c_state.value {:?}", symbol, nb_bits_out, c_state.value);
    }
}

#[inline]
fn fse_flush_cstate(
    bit_c: &mut BitCstream,
    c_state: &mut FseCState,
    table_log: u32,
    data: &mut [u8],
) {
    bit_c.add_bits(c_state.value, table_log);
    bit_c.flush_bits_fast(data);
}
