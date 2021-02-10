use crate::bitstream::BitCstream;
use crate::table::CompressionTable;
use crate::FSE_MAX_TABLELOG;

// FSE buffer bounds
pub const FSE_NCOUNTBOUND: usize = 512;

#[inline]
fn fse_blockbound(size: usize) -> usize {
    size + (size>>7) + 4 /* fse states */ + core::mem::size_of::<usize>() /* bitContainer */
}

#[inline]
fn fse_compressbound(size: usize) -> usize {
    FSE_NCOUNTBOUND + fse_blockbound(size)
}

#[inline]
pub fn fse_compress_using_ctable_generic(
    input: &[u8],
    comp: &CompressionTable,
    table_log: u32,
) -> BitCstream {
    assert!(input.len() > 2);
    let max_compressed_size = fse_compressbound(input.len());

    let mut bit_c = BitCstream::new(max_compressed_size);

    let mut index = input.len();

    let (mut state1, mut state2) = if input.len() & 1 == 1 {
        index -= 1;
        let mut state1 = FseCState::new(input[index], &comp);
        index -= 1;
        let state2 = FseCState::new(input[index], &comp);
        index -= 1;
        fse_encode_symbol(&mut bit_c, &mut state1, comp, input[index]);
        bit_c.flush_bits_fast();
        (state1, state2)
    } else {
        index -= 1;
        let state2 = FseCState::new(input[index], &comp);
        index -= 1;
        let state1 = FseCState::new(input[index], &comp);
        (state1, state2)
    };

    const BIT_CONTAINER_SIZE: usize = core::mem::size_of::<usize>();

    // join to mod 4
    if (BIT_CONTAINER_SIZE * 8 > FSE_MAX_TABLELOG as usize * 4 + 7)
        && (((input.len() - 2) & 2) == 2)
    // test bit 2
    {
        index -= 1;
        fse_encode_symbol(&mut bit_c, &mut state2, comp, input[index]);
        index -= 1;
        fse_encode_symbol(&mut bit_c, &mut state1, comp, input[index]);
        bit_c.flush_bits_fast();
    }

    // index+=1; // adding one, to avoid buffer overflow
    // while index > 0 {
    //     index-=1;
    //     fse_encode_symbol(&mut bit_c, &mut state2, comp, input[index]);
    //     index-=1;

    //     if BIT_CONTAINER_SIZE * 8 < FSE_MAX_TABLELOG as usize * 2 + 7 { // 64bit 64 < 24 + 7, 32bit 32 < 24 + 7
    //         bit_c.flush_bits_fast();
    //     }

    //     fse_encode_symbol(&mut bit_c, &mut state1, comp, input[index]);
    //     index-=1;

    //     if BIT_CONTAINER_SIZE * 8 > FSE_MAX_TABLELOG as usize * 4 + 7 { // 64bit 64 > 48 + 7, 32bit 32 > 48 + 7
    //         fse_encode_symbol(&mut bit_c, &mut state2, comp, input[index]);
    //         index-=1;
    //         fse_encode_symbol(&mut bit_c, &mut state1, comp, input[index]);
    //     }

    //     bit_c.flush_bits_fast();
    // }

    // these loops are correct for FSE_MAX_TABLELOG = 12

    #[cfg(target_pointer_width = "64")]
    {
        // 64 bit version
        for chunk in input[..index].rchunks_exact(4) {
            fse_encode_symbol(&mut bit_c, &mut state2, comp, chunk[0]);
            fse_encode_symbol(&mut bit_c, &mut state1, comp, chunk[1]);
            fse_encode_symbol(&mut bit_c, &mut state2, comp, chunk[2]);
            fse_encode_symbol(&mut bit_c, &mut state1, comp, chunk[3]);
            bit_c.flush_bits_fast();
        }
    }

    #[cfg(target_pointer_width = "32")]
    {
        // 32 bit version
        for chunk in input[..index].rchunks_exact(2) {
            fse_encode_symbol(&mut bit_c, &mut state2, comp, chunk[0]);
            bit_c.flush_bits_fast();
            fse_encode_symbol(&mut bit_c, &mut state1, comp, chunk[1]);
            bit_c.flush_bits_fast();
        }
    }

    fse_flush_cstate(&mut bit_c, &mut state2, table_log);
    fse_flush_cstate(&mut bit_c, &mut state1, table_log);

    bit_c.finish_stream();

    bit_c
}

#[inline]
fn fse_encode_symbol(
    bit_c: &mut BitCstream,
    c_state: &mut FseCState,
    comp: &CompressionTable,
    symbol: u8,
) {
    let symbol_tt = comp.symbol_tt[symbol as usize];
    let nb_bits_out: u32 = ((c_state.value + symbol_tt.deltaNbBits as usize) >> 16) as u32;
    bit_c.add_bits(c_state.value, nb_bits_out);
    c_state.value = comp.state_table
        [((c_state.value >> nb_bits_out) as isize + symbol_tt.deltaFindState as isize) as usize]
        as usize;

    // println!("symbol {:?} nb_bits_out {:?} c_state.value {:?}", symbol, nb_bits_out, c_state.value);
}

#[inline]
fn fse_flush_cstate(bit_c: &mut BitCstream, c_state: &mut FseCState, table_log: u32) {
    bit_c.add_bits(c_state.value, table_log);
}

#[derive(Debug)]
struct FseCState {
    value: usize,
}

impl FseCState {
    fn new(symbol: u8, comp: &CompressionTable) -> Self {
        let symbol_tt = comp.symbol_tt[symbol as usize];
        let nb_bits_out: u32 = ((symbol_tt.deltaNbBits as usize + (1 << 15)) >> 16) as u32;
        let value: usize = ((nb_bits_out as usize) << 16) - symbol_tt.deltaNbBits as usize;
        let value: usize = comp.state_table
            [((value >> nb_bits_out) as isize + symbol_tt.deltaFindState as isize) as usize]
            as usize;

        FseCState { value }
    }
}
