use crate::{FSE_MAX_TABLELOG, FSE_MIN_TABLELOG};

/// calculate recommended table_log size
pub fn fse_optimal_table_log(max_table_log: u32, src_size: usize, max_symbol_value: u32) -> u32 {
    // magic number minus 2, https://github.com/Cyan4973/FiniteStateEntropy/blob/5b3f8551695351d2a16d383c55bd7cddfd5c3813/lib/fse_compress.c#L341
    fse_optimal_table_log_interal(max_table_log, src_size, max_symbol_value, 2)
}
/// calculate recommended table_log size
pub fn fse_optimal_table_log_interal(
    max_table_log: u32,
    src_size: usize,
    max_symbol_value: u32,
    minus: u32,
) -> u32 {
    let mut table_log = max_table_log;

    let max_bits_src = highbit_pos(src_size as u32 - 1) - minus;
    let min_bits = fse_min_table_log(src_size, max_symbol_value);

    table_log = table_log.min(max_bits_src); // accuracy can be reduced
    table_log = table_log.max(min_bits); // Need a minimum to safely represent all symbol values

    table_log = table_log.min(FSE_MAX_TABLELOG);
    table_log = table_log.max(FSE_MIN_TABLELOG);

    table_log
}
/// provides the minimum log size to safely represent a distribution
pub fn fse_min_table_log(src_size: usize, max_symbol_value: u32) -> u32 {
    assert!(src_size > 1); // not supported
    let min_bits_src: u32 = highbit_pos(src_size as u32) + 1;
    let min_bits_symbols: u32 = highbit_pos(max_symbol_value) + 2;
    min_bits_src.min(min_bits_symbols)
}
/// returns the position of the highest bit
///
/// see test_highbit_pos
#[inline]
pub fn highbit_pos(val: u32) -> u32 {
    return val.leading_zeros() ^ 31;
}

#[test]
fn test_highbit_pos() {
    assert_eq!(highbit_pos(1), 0);
    assert_eq!(highbit_pos(2), 1);
    assert_eq!(highbit_pos(4), 2);
    assert_eq!(highbit_pos(7), 2);
    assert_eq!(highbit_pos(8), 3);
    assert_eq!(highbit_pos(9), 3);
    assert_eq!(highbit_pos(1000), 9);
    assert_eq!(highbit_pos(1024), 10);
}
