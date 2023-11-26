mod error;
mod table;
use std::convert::TryInto;

use error::HistError;
use log::log_enabled;
use log::Level::Trace;
use log::*;
pub use table::fse_optimal_table_log;

// use crate::table::fse_min_table_log;

pub const MAX_SYMBOL_VALUE: u32 = u8::MAX as u32;

pub type CountsTable = [u32; MAX_SYMBOL_VALUE as usize + 1];
pub type NormCountsTable = [i16; MAX_SYMBOL_VALUE as usize + 1];

pub fn get_max_symbol_value(counts: &CountsTable) -> u32 {
    let mut max_symbol_value = MAX_SYMBOL_VALUE;

    while counts[max_symbol_value as usize] == 0 {
        max_symbol_value -= 1;
    }

    max_symbol_value
}

/// Normalize the frequencies.
///
/// get_normalized_counts() will ensure that sum of frequencies is == 2 ^ tableLog.
#[inline]
pub fn get_normalized_counts(
    counts: &CountsTable,
    table_log: u32,
    src_size: usize,
    max_symbol_value: u32,
) -> NormCountsTable {
    debug!("table_log: {:?}", table_log);
    // if table_log < fse_min_table_log(src_size, max_symbol_value) {
    //     panic!("Too small tableLog, compression potentially impossible table_log {:?} fse_min_table_log {:?} ", table_log, fse_min_table_log(src_size, max_symbol_value));
    // };

    let total = src_size as u64;
    // Variable length arrays are not yet supported in Rust, [0_i16;max_symbol_value] would be enough for the counts;
    // https://doc.rust-lang.org/beta/unstable-book/language-features/unsized-locals.html
    // This should also remove bounds checks for the loop below.
    let mut norm_counts = [0_i16; 256];

    /// rest to beat table
    const RTB_TABLE: [u32; 8] = [0, 473195, 504333, 520860, 550000, 700000, 750000, 830000];

    let scale: u64 = 62 - table_log as u64;

    let step: u64 = (1 << 62) / total;
    let v_step: u64 = 1 << (scale - 20);

    // keeps track of the sum of occurences of symbols to match 1 << table_log
    let mut still_to_distribute: i32 = 1 << table_log;

    let mut largest: u32 = 0;
    let mut largest_p: i16 = 0;

    let low_threshold = (total >> table_log) as u32;
    for symbol in 0..=max_symbol_value as usize {
        let symbol_count = counts[symbol];

        // this is not yet supported, rle case
        assert!(symbol_count as u64 != total);
        if symbol_count == 0 {
            continue;
        }
        if symbol_count < low_threshold {
            norm_counts[symbol] = -1;
            still_to_distribute -= 1;
        } else {
            let mut proba: i16 = ((symbol_count as u64 * step) >> scale) as i16;
            if proba < 8 {
                let rest_to_beat = v_step * RTB_TABLE[proba as usize] as u64;
                if (symbol_count as u64 * step) - ((proba as u64) << scale) > rest_to_beat {
                    proba += 1;
                }
            }

            if proba > largest_p {
                largest_p = proba;
                largest = symbol as u32;
            }
            norm_counts[symbol] = proba;
            still_to_distribute -= proba as i32;
        }
    }

    debug!("still_to_distribute: {}", still_to_distribute);
    if -still_to_distribute >= (norm_counts[largest as usize] as i32 >> 1) {
        panic!("count normalization corner case not yet implemented");
        /* corner case, need another normalization method */
        // size_t const errorCode = FSE_normalizeM2(norm_counts, tableLog, count, total, maxSymbolValue);
        // if (FSE_isError(errorCode)) return errorCode;
    } else {
        // assign rest to match total norm counts = 1 << table_log
        norm_counts[largest as usize] += still_to_distribute as i16;
    }

    if log_enabled!(Trace) {
        let mut n_total = 0;
        for symbol in 0..=max_symbol_value as usize {
            if norm_counts[symbol] != 0 {
                trace!("{}: {}", symbol, norm_counts[symbol]);
            }
            n_total += norm_counts[symbol].abs();
        }

        if n_total != (1 << table_log) {
            error!("Warning Total {} != table_log {}", n_total, 1 << table_log);
        }
    }

    norm_counts
}

/// creates a table with the counts of each symbol
#[inline]
pub fn count_simple(input: &[u8]) -> CountsTable {
    let mut counts = [0_u32; 256];

    for byte in input {
        counts[*byte as usize] = counts[*byte as usize].saturating_add(1);
    }
    counts
}

pub fn get_normalized_counts_from_data(data: &[u8]) -> (NormCountsTable, u32, u32) {
    let counts = count_simple(data);
    let max_symbol_value = get_max_symbol_value(&counts);
    let table_log = fse_optimal_table_log(FSE_DEFAULT_TABLELOG, data.len(), max_symbol_value);

    let norm_counts = get_normalized_counts(&counts, table_log, data.len(), max_symbol_value);
    (norm_counts, max_symbol_value, table_log)
}

/// creates a table with the counts of each symbol
#[inline]
pub fn count_multi(input: &[u8]) -> CountsTable {
    let mut counts1 = [0_u32; 256];
    let mut counts2 = [0_u32; 256];
    let mut counts3 = [0_u32; 256];
    let mut counts4 = [0_u32; 256];
    // let mut counts5 = [0_u32; 256];
    // let mut counts6 = [0_u32; 256];
    // let mut counts7 = [0_u32; 256];
    // let mut counts8 = [0_u32; 256];

    let offset = input.as_ptr().align_offset(core::mem::align_of::<usize>());
    let (left, right) = input.split_at(offset);
    for el in left {
        counts1[*el as usize] += 1;
    }

    let mut iter = right.chunks_exact(8);
    for chunks in &mut iter {
        counts1[chunks[0] as usize] = counts1[chunks[0] as usize].saturating_add(1);
        counts2[chunks[1] as usize] = counts2[chunks[1] as usize].saturating_add(1);
        counts3[chunks[2] as usize] = counts3[chunks[2] as usize].saturating_add(1);
        counts4[chunks[3] as usize] = counts4[chunks[3] as usize].saturating_add(1);
        counts1[chunks[4] as usize] = counts1[chunks[4] as usize].saturating_add(1);
        counts2[chunks[5] as usize] = counts2[chunks[5] as usize].saturating_add(1);
        counts3[chunks[6] as usize] = counts3[chunks[6] as usize].saturating_add(1);
        counts4[chunks[7] as usize] = counts4[chunks[7] as usize].saturating_add(1);
        // counts1[chunks[8] as usize] += 1;
        // counts2[chunks[9] as usize] += 1;
        // counts3[chunks[10] as usize] += 1;
        // counts4[chunks[11] as usize] += 1;
        // counts5[chunks[12] as usize] += 1;
        // counts6[chunks[13] as usize] += 1;
        // counts7[chunks[14] as usize] += 1;
        // counts8[chunks[15] as usize] += 1;
    }

    for el in iter.remainder() {
        counts1[*el as usize] += 1;
    }

    let iter = counts1
        .iter_mut()
        .zip(counts2.iter().zip(counts3.iter().zip(counts4.iter())));

    for (el1, (el2, (el3, el4))) in iter {
        *el1 += *el2 + *el3 + *el4;
    }

    // let iter = counts1.iter_mut().zip(counts2.iter_mut().zip(counts3.iter_mut().zip(counts4.iter_mut().zip(counts5.iter_mut().zip(counts6.iter_mut().zip(counts7.iter_mut().zip(counts8.iter_mut())))))));

    // for (el1, (el2, (el3, (el4, (el5, (el6, (el7, el8))))))) in iter {
    //     *el1 += *el2 + *el3 + *el4 + *el5 + *el6 + *el7 + *el8 ;
    // }

    counts1
}
/// creates a table with the counts of each symbol
#[inline]
pub fn count_blocked_unsafe(input: &[u8]) -> Vec<u32> {
    let mut counts = vec![0_u32; 256 * 4];
    let rest = counts.as_mut_slice();
    let (counts1, rest) = rest.split_at_mut(256);
    let (counts2, rest) = rest.split_at_mut(256);
    let (counts3, counts4) = rest.split_at_mut(256);
    // let mut counts2 = [0_u32; 256];
    // let mut counts3 = [0_u32; 256];
    // let mut counts4 = [0_u32; 256];

    unsafe {
        let mut in_ptr = input.as_ptr();
        let iend = input.as_ptr().add(input.len());
        let offset = input.as_ptr().align_offset(core::mem::align_of::<usize>());

        for _ in 0..offset {
            let val = core::ptr::read::<u8>(in_ptr);
            counts1[val as usize] += 1;
            in_ptr = in_ptr.add(1);
        }

        while (in_ptr as usize) < iend as usize - 16 {
            let val = core::ptr::read::<u32>(in_ptr as *const u32);
            counts1[(val as u8) as usize] += 1;
            counts2[((val >> 8) as u8) as usize] += 1;
            counts3[((val >> 16) as u8) as usize] += 1;
            counts4[((val >> 24) as u8) as usize] += 1;
            in_ptr = in_ptr.add(4);
            let val = core::ptr::read::<u32>(in_ptr as *const u32);
            counts1[(val as u8) as usize] += 1;
            counts2[((val >> 8) as u8) as usize] += 1;
            counts3[((val >> 16) as u8) as usize] += 1;
            counts4[((val >> 24) as u8) as usize] += 1;
            in_ptr = in_ptr.add(4);

            let val = core::ptr::read::<u32>(in_ptr as *const u32);
            counts1[(val as u8) as usize] += 1;
            counts2[((val >> 8) as u8) as usize] += 1;
            counts3[((val >> 16) as u8) as usize] += 1;
            counts4[((val >> 24) as u8) as usize] += 1;
            in_ptr = in_ptr.add(4);
            let val = core::ptr::read::<u32>(in_ptr as *const u32);
            counts1[(val as u8) as usize] += 1;
            counts2[((val >> 8) as u8) as usize] += 1;
            counts3[((val >> 16) as u8) as usize] += 1;
            counts4[((val >> 24) as u8) as usize] += 1;
            in_ptr = in_ptr.add(4);
        }

        while in_ptr < iend {
            let val = core::ptr::read::<u8>(in_ptr);
            counts1[val as usize] += 1;
            in_ptr = in_ptr.add(1);
        }
    }

    let iter = counts1
        .iter_mut()
        .zip(counts2.iter().zip(counts3.iter().zip(counts4.iter())));

    for (el1, (el2, (el3, el4))) in iter {
        *el1 += *el2 + *el3 + *el4;
    }

    // let iter = counts1.iter_mut().zip(counts2.iter_mut().zip(counts3.iter_mut().zip(counts4.iter_mut().zip(counts5.iter_mut().zip(counts6.iter_mut().zip(counts7.iter_mut().zip(counts8.iter_mut())))))));

    // for (el1, (el2, (el3, (el4, (el5, (el6, (el7, el8))))))) in iter {
    //     *el1 += *el2 + *el3 + *el4 + *el5 + *el6 + *el7 + *el8 ;
    // }

    counts.resize(256, 0);
    counts
}

pub const FSE_MIN_TABLELOG: u32 = 5;
pub const FSE_NCOUNTBOUND: u32 = 512;

pub const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
pub const FSE_MAX_MEMORY_USAGE: u32 = 14; // 16kb
pub const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;

pub const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;
pub const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
pub const FSE_MAX_TABLESIZE: usize = 1 << FSE_MAX_TABLELOG;
pub const FSE_MAXTABLESIZE_MASK: usize = FSE_MAX_TABLESIZE - 1;
pub const FSE_MAX_SYMBOL_VALUE: u32 = u8::MAX as u32;

pub const HIST_WKSP_SIZE_U32: usize = 1024;
pub const HIST_WKSP_SIZE: usize = HIST_WKSP_SIZE_U32 * core::mem::size_of::<usize>();

pub fn fse_NCountWriteBound(max_symbol_value: u32, table_log: u32) -> u32 {
    let max_header_size = (((max_symbol_value + 1) * table_log) >> 3) + 3;
    if max_symbol_value == 0 {
        FSE_NCOUNTBOUND
    } else {
        max_header_size
    }
}

/// write count metadata into header which is used by FSE and hufmann
pub fn FSE_write_N_Count(
    out: &mut [u8],
    norm_counts: &NormCountsTable,
    max_symbol_value: u32,
    table_log: u32,
) -> Result<usize, HistError> {
    if table_log > FSE_MAX_TABLELOG {
        return Err(HistError::TableLogTooLarge);
    }

    if table_log < FSE_MIN_TABLELOG {
        return Err(HistError::TableLogTooSmall);
    }
    if out.len() < fse_NCountWriteBound(max_symbol_value, table_log) as usize {
        fse_write_n_count_generic(out, norm_counts, max_symbol_value, table_log, false)
    } else {
        fse_write_n_count_generic(out, norm_counts, max_symbol_value, table_log, true)
    }
}

/// write count metadata into header which is used by FSE and hufmann
pub fn fse_write_n_count_generic(
    mut out: &mut [u8],
    norm_counts: &NormCountsTable,
    max_symbol_value: u32,
    table_log: u32,
    write_is_safe: bool,
) -> Result<usize, HistError> {
    let out_len = out.len();
    let table_size = 1 << table_log;
    let mut nb_bits = table_log + 1; // + 1 for extra accuracy
    let mut remaining: i32 = table_size + 1;
    let mut threshold = table_size;

    let mut bit_stream: u32 = table_log - FSE_MIN_TABLELOG;
    let mut bit_count = 4;

    let mut previous_is0 = false;

    let mut symbol: u32 = 0;
    let alphabet_size = max_symbol_value + 1;
    while symbol < alphabet_size && remaining > 1 {
        if previous_is0 {
            let mut start = symbol;
            while symbol < alphabet_size && norm_counts[symbol as usize] == 0 {
                symbol += 1;
            }
            if symbol == alphabet_size {
                break; // incorrect distribution
            }
            while symbol >= start + 24 {
                start += 24;
                bit_stream += 0xFFFF << bit_count;
                if !write_is_safe && out.len() < 2 {
                    return Err(HistError::OutputTooSmall);
                }
                out[0] = bit_stream as u8;
                out[1] = (bit_stream >> 8) as u8;
                out = &mut out[2..];
                bit_stream >>= 16;
            }
            while symbol >= start + 3 {
                start += 3;
                bit_stream += 3 << bit_count;
                bit_count += 2;
            }
            bit_stream += (symbol - start) << bit_count;
            bit_count += 2;
            if bit_count > 16 {
                if !write_is_safe && out.len() < 2 {
                    return Err(HistError::OutputTooSmall);
                }
                out[0] = bit_stream as u8;
                out[1] = (bit_stream >> 8) as u8;
                out = &mut out[2..];
                bit_stream >>= 16;
                bit_count -= 16;
            }
        }
        let mut count = norm_counts[symbol as usize] as i32;
        symbol += 1;
        let max = (2 * threshold - 1) - remaining;
        remaining -= count.abs();
        count += 1;
        if count >= threshold {
            count += max;
        }
        bit_stream += (count as u32) << bit_count;
        bit_count += nb_bits;
        if count < max {
            bit_count -= 1;
        }
        previous_is0 = count == 1;
        if remaining < 1 {
            return Err(HistError::UnexpectedRemaining);
        }
        while remaining < threshold {
            nb_bits -= 1;
            threshold >>= 1;
        }

        if bit_count > 16 {
            if !write_is_safe && out.len() < 2 {
                return Err(HistError::OutputTooSmall);
            }
            out[0] = bit_stream as u8;
            out[1] = (bit_stream >> 8) as u8;
            out = &mut out[2..];
            bit_stream >>= 16;
            bit_count -= 16;
        }
    }
    assert!(symbol <= alphabet_size);
    if remaining != 1 {
        return Err(HistError::IncorrectNormalizedDistribution);
    }
    if !write_is_safe && out.len() < 2 {
        return Err(HistError::OutputTooSmall);
    }
    out[0] = bit_stream as u8;
    out[1] = (bit_stream >> 8) as u8;
    out = &mut out[(bit_count as usize + 7) / 8..];
    let bytes_written = out_len - out.len();
    Ok(bytes_written)
}

/// write count metadata into header which is used by FSE and hufmann
pub fn fse_read_n_count(
    mut data: &[u8],
    norm_counts: &mut NormCountsTable,
    max_symbol_value: &mut u32,
    table_log: &mut u32,
) -> Result<usize, HistError> {
    let data_len = data.len();
    if data.len() < 4 {
        let mut buffer = [0, 0, 0, 0];
        buffer[..data.len()].copy_from_slice(data);
        return fse_read_n_count(data, norm_counts, max_symbol_value, table_log);
    }

    let mut bit_stream = u32::from_le_bytes(data[..4].try_into().unwrap());
    let mut nb_bits = (bit_stream & 0xF) + FSE_MIN_TABLELOG; // extract table_log
    if nb_bits > FSE_TABLELOG_ABSOLUTE_MAX {
        return Err(HistError::TableLogTooLarge);
    }
    bit_stream >>= 4;
    let mut bit_count: i32 = 4;
    *table_log = nb_bits;
    let mut remaining = (1 << nb_bits) + 1;
    let mut threshold: i32 = 1 << nb_bits;
    nb_bits += 1;

    let mut previous_is0 = false;
    let mut charnum = 0_u32;
    while remaining > 1 && charnum <= *max_symbol_value {
        if previous_is0 {
            let mut n0: u32 = charnum;
            while bit_stream & 0xFFFF == 0xFFFF {
                // check if all first 16 bytes are all 1
                n0 += 24;
                if data.len() > 5 {
                    data = &data[2..];
                    bit_stream = u32::from_le_bytes(data[..4].try_into().unwrap()) >> bit_count;
                } else {
                    bit_stream >>= 16;
                    bit_count += 16;
                }
            }
            while (bit_stream & 3) == 3 {
                n0 += 3;
                bit_stream >>= 2;
                bit_count += 2;
            }
            n0 += bit_stream & 3;
            bit_count += 2;
            if n0 > *max_symbol_value {
                return Err(HistError::MaxSymbolValueTooSmall);
            }
            while charnum < n0 {
                norm_counts[charnum as usize] = 0;
                charnum += 1;
            }
            if data.len() >= 7 || data.len() - (bit_count as usize >> 3) >= 4 {
                assert!(bit_count >> 3 <= 3);
                data = &data[bit_count as usize >> 3..];
                bit_count &= 7;
                bit_stream = u32::from_le_bytes(data[..4].try_into().unwrap()) >> bit_count;
            } else {
                bit_stream >>= 2;
            }
        }
        let max = (2 * threshold - 1) - remaining;
        let mut count = if (bit_stream as i32 & (threshold - 1)) < max {
            bit_count += nb_bits as i32 - 1;
            bit_stream as i32 & (threshold - 1)
        } else {
            let mut count = bit_stream as i32 & (2 * threshold - 1);
            if count >= threshold {
                count -= max;
            }
            bit_count += nb_bits as i32;
            count
        };
        count -= 1; // extra accuracy
        remaining -= count.abs(); // abs to convert -1 special case to +1
        norm_counts[charnum as usize] = count as i16;
        charnum += 1;
        previous_is0 = count == 0;
        while remaining < threshold {
            nb_bits -= 1;
            threshold >>= 1;
        }
        if data.len() >= 7 || data.len() - (bit_count as usize >> 3) >= 4 {
            data = &data[bit_count as usize >> 3..];
            bit_count &= 7;
        } else {
            bit_count -= (8 * data.len() - 4) as i32;
            data = &data[..data.len() - 4]; // could be an issue if data has less than 4 left?
        }
        bit_stream = u32::from_le_bytes(data[..4].try_into().unwrap()) >> (bit_count & 31);
    }
    if remaining != 1 {
        return Err(HistError::CorruptionDetected(
            "remaining is not 1, but ".to_string() + &remaining.to_string(),
        ));
    }
    if bit_count > 32 {
        return Err(HistError::CorruptionDetected("bit_count > 32".to_string()));
    }
    *max_symbol_value = charnum - 1;

    data = &data[(bit_count as usize + 7) >> 3..]; // could be an issue if data has less than 4 left?
    let bytes_read = data_len - data.len();
    Ok(bytes_read)
}

#[cfg(test)]
mod tests {

    use super::count_blocked_unsafe;
    use super::count_multi;
    use super::count_simple;
    use super::get_normalized_counts;
    use super::*;

    const A_BYTE: u8 = "a".as_bytes()[0];
    const B_BYTE: u8 = "b".as_bytes()[0];
    const C_BYTE: u8 = "c".as_bytes()[0];

    fn get_test_data() -> Vec<u8> {
        use std::io::Read;
        let mut buffer = Vec::new();
        std::io::repeat(A_BYTE)
            .take(45)
            .read_to_end(&mut buffer)
            .unwrap(); // 45% prob
        std::io::repeat(B_BYTE)
            .take(35)
            .read_to_end(&mut buffer)
            .unwrap(); // 35% prob
        std::io::repeat(C_BYTE)
            .take(20)
            .read_to_end(&mut buffer)
            .unwrap(); // 20% prob

        buffer
    }
    #[test]
    #[ignore]
    fn write_n_bound_test() -> Result<(), HistError> {
        let test_data: &[u8] = &[
            3_u8, 4, 4, 6, 50, 51, 51, 51, 51, 52, 52, 52, 52, 52, 52, 52, 52, 52,
        ];
        use std::io::Write;
        std::fs::File::create("../../FiniteStateEntropy/programs/test_data_100")
            .unwrap()
            .write_all(test_data)
            .unwrap();

        //let mut test_data = vec![];
        //use std::io::Read;
        //std::fs::File::open("../../FiniteStateEntropy/programs/test_data_100")
        //.unwrap()
        //.read_to_end(&mut test_data)
        //.unwrap();
        let (norm_counts, mut max_symbol_value, table_log) =
            get_normalized_counts_from_data(test_data);
        let mut out = vec![];
        out.resize(
            fse_NCountWriteBound(max_symbol_value, table_log) as usize,
            0,
        );
        let bytes_written = FSE_write_N_Count(
            out.as_mut_slice(),
            &norm_counts,
            max_symbol_value,
            table_log,
        )?;
        dbg!(bytes_written);
        dbg!(&out[..bytes_written]);
        let mut table_log_restored = FSE_DEFAULT_TABLELOG;
        let mut norm_counts_restored = [0_i16; 256];
        let _max_symbol_value_restored = FSE_MAX_SYMBOL_VALUE;
        fse_read_n_count(
            &out,
            &mut norm_counts_restored,
            &mut max_symbol_value,
            &mut table_log_restored,
        )?;
        assert_eq!(norm_counts, norm_counts_restored);
        Ok(())
    }

    #[test]
    fn test_statistic_fns() {
        let test_data = get_test_data();

        let counts = count_simple(&test_data);
        assert_eq!(counts[A_BYTE as usize], 45);
        assert_eq!(counts[B_BYTE as usize], 35);
        assert_eq!(counts[C_BYTE as usize], 20);

        let counts = count_blocked_unsafe(&test_data);
        assert_eq!(counts[A_BYTE as usize], 45);
        assert_eq!(counts[B_BYTE as usize], 35);
        assert_eq!(counts[C_BYTE as usize], 20);

        let counts = count_multi(&test_data);
        assert_eq!(counts[A_BYTE as usize], 45);
        assert_eq!(counts[B_BYTE as usize], 35);
        assert_eq!(counts[C_BYTE as usize], 20);

        let norm_counts = get_normalized_counts(&counts, 7, test_data.len(), 255);

        assert_eq!(norm_counts[A_BYTE as usize], 59);
        assert_eq!(norm_counts[B_BYTE as usize], 44);
        assert_eq!(norm_counts[C_BYTE as usize], 25);

        // make sure sum is power of 2 of table_log
        assert_eq!(
            norm_counts[A_BYTE as usize]
                + norm_counts[B_BYTE as usize]
                + norm_counts[C_BYTE as usize],
            128
        );

        let norm_counts = get_normalized_counts(&counts, 8, test_data.len(), 255);
        // make sure sum is power of 2 of table_log
        assert_eq!(
            norm_counts[A_BYTE as usize]
                + norm_counts[B_BYTE as usize]
                + norm_counts[C_BYTE as usize],
            2_i16.pow(8)
        );
    }
}
