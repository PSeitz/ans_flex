use crate::*;
use bitstream::highbit_pos;
use common::NormCountsTable;
use log::log_enabled;
use log::Level::{Debug, Trace};
use log::*;

#[derive(Debug, Clone)]
pub struct CompressionTable {
    pub state_table: Vec<u16>,
    pub symbol_tt: Vec<FseSymbolCompressionTransform>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FseSymbolCompressionTransform {
    pub delta_find_state: i32,
    pub delta_nb_bits: u32,
}

impl FseSymbolCompressionTransform {
    /// Approximate maximum cost of a symbol, in bits.
    ///
    /// Fractional get rounded up (i.e : a symbol with a normalized frequency of 3 gives the same result as a frequency of 2)
    /// note 1 : assume symbolValue is valid (<= maxSymbolValue)
    /// note 2 : if freq[symbolValue]==0, @return a fake cost of tableLog+1 bits */
    pub fn fse_get_max_nb_bits(&self) -> u32 {
        (self.delta_nb_bits + ((1 << 16) - 1)) >> 16
    }

    /// Approximate symbol cost, as fractional value, using fixed-point format (accuracy_log fractional bits)
    ///
    /// note 1 : assume symbolValue is valid (<= maxSymbolValue)
    /// note 2 : if freq[symbolValue]==0, @return a fake cost of tableLog+1 bits */
    pub fn fse_bit_cost(&self, table_log: u32, accuracy_log: u32) -> u32 {
        let min_nb_bits: u32 = self.delta_nb_bits >> 16;
        let threshold: u32 = (min_nb_bits + 1) << 16;

        assert!(table_log < 16);
        assert!(accuracy_log < 31 - table_log);
        let table_size = 1 << table_log;

        let delta_from_threshold: u32 = threshold - (self.delta_nb_bits + table_size);
        let normalized_delta_from_threshold: u32 =
            (delta_from_threshold << accuracy_log) >> table_log; /* linear interpolation (very approximate) */
        let bit_multiplier: u32 = 1 << accuracy_log;
        assert!(self.delta_nb_bits + table_size <= threshold);
        assert!(normalized_delta_from_threshold <= bit_multiplier);
        (min_nb_bits + 1) * bit_multiplier - normalized_delta_from_threshold
    }
}

#[test]
fn test_table_log_limit() {
    // Max value of min bits required imposed by FSE_MAX_SYMBOL_VALUE (too many min max :)
    let min_bits_symbols: u32 = highbit_pos(FSE_MAX_SYMBOL_VALUE) + 2;
    assert_eq!(min_bits_symbols, 9);

    // make sure the upper bound FSE_MAX_TABLELOG is not smaller than the upper bound imposed by number of symbols
    assert!(min_bits_symbols < FSE_MAX_TABLELOG);
}

/// Creating an ANSTable consists of following steps
///
/// 1. count symbol occurrence from input[] into table count[]
/// 2. normalize counters so that sum(count[]) == 2^table_log
/// 3. build encoding table 'CompressionTable' from normalized counters
///
/// build_table is step 3
///
pub fn build_compression_table(
    norm_counts: &NormCountsTable,
    table_log: u32,
    mut max_symbol_value: u32,
) -> CompressionTable {
    let table_size = 1 << table_log;
    debug!("table_size {:?}", table_size);
    let table_mask = table_size - 1;
    let mut high_threshold = table_size - 1;
    max_symbol_value = max_symbol_value.min(FSE_MAX_SYMBOL_VALUE);
    let mut cumul = vec![0_u32; max_symbol_value as usize + 2];

    // tmp table - TODO Could be externally allocated and reused
    // This is the classical table symbol table, where the state equals its position in the table
    // In the classical approach, they are illustrated like this
    // State    A    B    C
    // 1        2    3    5
    // 2        4    6    10
    // 3        7    8    15
    //
    let mut table_symbol = vec![0_u8; table_size];

    // out table
    // get_ans_table_size will return usually a smaller value that table_size
    // Currently not clear why - 05.02.2021
    // let mut compression_table = vec![0_u32, get_ans_table_size(table_log, max_symbol_value)];

    // symbol start positions
    debug!("max_symbol_value{:?}", max_symbol_value);
    for u in 1..=max_symbol_value as usize + 1 {
        if norm_counts[u - 1] == -1 {
            // Low proba symbol
            cumul[u] = cumul[u - 1] + 1;
            table_symbol[high_threshold] = (u - 1) as u8;
            high_threshold -= 1;
        } else {
            cumul[u] = cumul[u - 1] + norm_counts[u - 1] as u32;
        }
        // print!("{:?}, ", cumul[u]);
        trace!("{:?}", cumul[u]);
    }
    cumul[max_symbol_value as usize + 1] = table_size as u32 + 1;
    trace!("{:?}", cumul[max_symbol_value as usize + 1]);

    // Spread symbols int the symbol table
    // the distribution is not perfect, but close enough
    {
        let step = fse_tablestep(table_size);
        let mut position = 0;
        for symbol in 0..=max_symbol_value {
            let freq = norm_counts[symbol as usize];
            for _ in 0..freq {
                table_symbol[position] = symbol as u8;
                position = (position + step) & table_mask;
                while position > high_threshold {
                    position = (position + step) & table_mask; // Low proba area
                }
            }
        }

        if log_enabled!(Trace) {
            for position in 0..table_size {
                trace!("table_symbol[{:?}] {:?}", position, table_symbol[position]);
            }
        }

        assert!(position == 0);
    }

    let mut state_table = vec![0_u16; cumul[max_symbol_value as usize + 1] as usize];
    // Build Table
    {
        for u in 0..table_size {
            let s = table_symbol[u];
            state_table[cumul[s as usize] as usize] = table_size as u16 + u as u16; // state_table : sorted by symbol order; gives next state value
            cumul[s as usize] += 1;
        }

        if log_enabled!(Trace) {
            for (position, val) in state_table.iter().enumerate() {
                trace!("state_table[{:?}] {:?}", position, val);
            }
            // trace!("state_table[{:?}] {:?}", 0, state_table[0]);
        }
    }

    // The symbol transformation table will help encoding input streams
    let mut symbol_tt =
        vec![FseSymbolCompressionTransform::default(); max_symbol_value as usize + 1];

    // Build Symbol Transformation Table
    {
        let mut total = 0_i32;
        for symbol in 0..=max_symbol_value as usize {
            let norm_count = norm_counts[symbol];
            match norm_count {
                0 => {
                    if log_enabled!(Debug) {
                        // For compatibility with fse_get_max_nb_bits()
                        symbol_tt[symbol].delta_nb_bits =
                            ((table_log + 1) << 16) - (1 << table_log)
                    }
                }
                -1 | 1 => {
                    symbol_tt[symbol].delta_nb_bits = (table_log << 16) - (1 << table_log);
                    symbol_tt[symbol].delta_find_state = total - 1;
                    total += 1;
                }
                _ => {
                    let max_bits_out: u32 = table_log - highbit_pos(norm_counts[symbol] as u32 - 1);
                    let min_state_plus: u32 = (norm_counts[symbol] as u32) << max_bits_out;
                    symbol_tt[symbol].delta_nb_bits = (max_bits_out << 16) - min_state_plus;
                    symbol_tt[symbol].delta_find_state = total - norm_counts[symbol] as i32;
                    total += norm_counts[symbol] as i32;
                    // trace!(
                    //     "symbol_tt[{:?}] delta_nb_bits {:?} delta_find_state {:?}",
                    //     symbol,
                    //     symbol_tt[symbol].delta_nb_bits,
                    //     symbol_tt[symbol].delta_find_state
                    // );
                }
            }
        }
    }

    if log_enabled!(Debug) {
        for symbol in 0..=max_symbol_value as usize {
            let weight = norm_counts[symbol];
            if weight == 0 {
                continue;
            }
            let s_tt = symbol_tt[symbol];
            debug!(
                "symbol:{:?} w:{:?} delta_nb_bits:{:?} delta_find_state:{:?} maxBits:{:?} fracBits:{:?} ",
                symbol,
                weight,
                s_tt.delta_nb_bits,
                s_tt.delta_find_state,
                s_tt.fse_get_max_nb_bits(),
                s_tt.fse_bit_cost(table_log, 8) as f32 / 256.0
            );
        }
    }

    CompressionTable {
        state_table,
        symbol_tt,
    }
}

#[derive(Debug)]
pub struct DecompressionTable {
    pub table: Vec<FseDecode>,
    pub fast: bool,
}

// pub type DecompressionTable = Vec<FseDecode>;

/// Build decoding table from normalized counters
pub fn build_decompression_table(
    norm_counts: &NormCountsTable,
    table_log: u32, // can be u8
    max_symbol_value: u32,
) -> DecompressionTable {
    let mut next_symbol_table = vec![0_u16; max_symbol_value as usize + 1];
    let table_size = 1 << table_log;
    let mut high_threshold = table_size - 1;
    let mut table_decode = vec![FseDecode::default(); table_size];

    assert!(max_symbol_value <= FSE_MAX_SYMBOL_VALUE);
    assert!(table_log <= FSE_MAX_TABLELOG);

    // build next_symbol_table
    let large_limit: i16 = (1 << (table_log - 1)) as i16;
    let mut fast_mode = true;
    for symbol in 0..=max_symbol_value as usize {
        let norm_count = norm_counts[symbol];
        if norm_count == -1 {
            table_decode[high_threshold].symbol = symbol as u8;
            high_threshold -= 1;
            next_symbol_table[symbol] = 1;
        } else {
            if norm_count > large_limit {
                fast_mode = false;
            }
            next_symbol_table[symbol] = norm_count as u16;
        }
    }

    // spread symbols - TODO basically the same as in compression
    {
        let table_mask = table_size - 1;
        let step = fse_tablestep(table_size);
        let mut position = 0;
        for symbol in 0..=max_symbol_value {
            let freq = norm_counts[symbol as usize];
            for _ in 0..freq {
                table_decode[position].symbol = symbol as u8;
                position = (position + step) & table_mask;
                while position > high_threshold {
                    position = (position + step) & table_mask; // Low proba area
                }
            }
        }

        if log_enabled!(Trace) {
            for position in 0..table_size {
                trace!(
                    "table_decode[{:?}] {:?}",
                    position,
                    table_decode[position].symbol
                );
            }
        }

        assert!(position == 0);
    }

    for u in 0..table_size {
        let decode_state = &mut table_decode[u];
        let symbol = decode_state.symbol as usize;
        let next_state = next_symbol_table[symbol];

        // state used, increment state
        next_symbol_table[symbol] += 1;
        decode_state.nb_bits = table_log as u8 - highbit_pos(next_state as u32) as u8;
        decode_state.new_state = (next_state << decode_state.nb_bits) - table_size as u16;
    }

    DecompressionTable {
        table: table_decode,
        fast: fast_mode,
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FseDecode {
    pub new_state: u16,
    pub symbol: u8,
    pub nb_bits: u8,
}
