use log::log_enabled;
use log::Level::Trace;
use log::*;

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
        if (symbol_count as u32) < low_threshold {
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
            counts1[(val as usize)] += 1;
            in_ptr = in_ptr.add(1);
        }

        while (in_ptr as usize) < iend as usize - 16 {
            let val = core::ptr::read::<u32>(in_ptr as *const u32);
            counts1[((val as u8) as usize)] += 1;
            counts2[(((val >> 8) as u8) as usize)] += 1;
            counts3[(((val >> 16) as u8) as usize)] += 1;
            counts4[(((val >> 24) as u8) as usize)] += 1;
            in_ptr = in_ptr.add(4);
            let val = core::ptr::read::<u32>(in_ptr as *const u32);
            counts1[((val as u8) as usize)] += 1;
            counts2[(((val >> 8) as u8) as usize)] += 1;
            counts3[(((val >> 16) as u8) as usize)] += 1;
            counts4[(((val >> 24) as u8) as usize)] += 1;
            in_ptr = in_ptr.add(4);

            let val = core::ptr::read::<u32>(in_ptr as *const u32);
            counts1[((val as u8) as usize)] += 1;
            counts2[(((val >> 8) as u8) as usize)] += 1;
            counts3[(((val >> 16) as u8) as usize)] += 1;
            counts4[(((val >> 24) as u8) as usize)] += 1;
            in_ptr = in_ptr.add(4);
            let val = core::ptr::read::<u32>(in_ptr as *const u32);
            counts1[((val as u8) as usize)] += 1;
            counts2[(((val >> 8) as u8) as usize)] += 1;
            counts3[(((val >> 16) as u8) as usize)] += 1;
            counts4[(((val >> 24) as u8) as usize)] += 1;
            in_ptr = in_ptr.add(4);
        }

        while in_ptr < iend {
            let val = core::ptr::read::<u8>(in_ptr);
            counts1[(val as usize)] += 1;
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

#[cfg(test)]
mod tests {

    use super::count_blocked_unsafe;
    use super::count_multi;
    use super::count_simple;
    use super::get_normalized_counts;

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
