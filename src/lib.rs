

#[derive(Debug)]
struct Counts {
    counts: [usize;256],
    total: usize,
}


/// creates a table with the counts of each symbol
pub fn count_simple(input: &[u8]) -> [usize;256] {
    let mut counts = [0;256];

    for byte in input {
        counts[*byte as usize] += 1
    }
    counts
}

fn create_ans_table(input: &[u8]) -> Vec<u32> {
    let counts = count_simple(&input);
    let max_val = get_table_max_val(&counts);

    let col_heights = get_column_heights(&counts);

    unimplemented!()
}


fn get_column_heights(counts: &[usize;256]) -> Vec<u32> {

    let max_val = get_table_max_val(&counts);
    let total = get_num_symbols(&counts);

    let sorted_counts = get_sorted_symbols(&counts);

    let mut is_first = true; // first == most probable
    let column_heigths = sorted_counts.iter().map(|entry|{
        let prob = counts[entry.symbol as usize] as f32 / total as f32;
        let mut val = (max_val as f32 * prob).floor() as u32;

        if is_first {
            is_first = false;
            val+=1;
        }
        val
    }).collect::<Vec<_>>();

    column_heigths
}

fn get_most_probable_symbol(counts: &[usize;256]) -> u8 {
    get_sorted_symbols(&counts)[0].symbol
}

#[derive(Debug)]
struct SymbolAndCount {
    symbol: u8,
    count: usize
}

fn get_sorted_symbols(counts: &[usize;256]) -> Vec<SymbolAndCount> {
    let mut symbols = counts.into_iter().enumerate().filter(|(_, val)|**val!=0).map(|(symbol, val)|SymbolAndCount{symbol: symbol as u8, count: *val}).collect::<Vec<_>>();

    // symbols.sort_by(|symb_cnt| symb_cnt.count);
    symbols.sort_by(|a, b| b.count.cmp(&a.count));

    symbols
}

fn get_table_max_val(counts: &[usize;256]) -> u32 {
    // magic_extra_bits is some value between 2 and 8
    // the higher the value, the better the compression, but it costs performance
    let magic_extra_bits = 4;

    let num_symbols = get_num_unique_symbols(&counts);
    let num_precision_bits = (num_symbols as f32).log2() as u32 + magic_extra_bits;
    let max_val = 2_u32.pow(num_precision_bits) - 1;
    max_val
}

fn get_num_unique_symbols(counts: &[usize;256]) -> usize {
    counts.into_iter().filter(|el|**el!=0).count()
}

fn get_num_symbols(counts: &[usize;256]) -> usize {
    counts.into_iter().sum()
}

#[cfg(test)]
mod tests {

    use super::*;

    const A_BYTE: u8 = "a".as_bytes()[0];
    const B_BYTE: u8 = "b".as_bytes()[0];
    const C_BYTE: u8 = "c".as_bytes()[0];

    fn get_test_data() -> Vec<u8> {
        use std::io::Read;
        let mut buffer = Vec::new();
        std::io::repeat(A_BYTE).take(45).read_to_end(&mut buffer).unwrap(); // 45% prob
        std::io::repeat(B_BYTE).take(35).read_to_end(&mut buffer).unwrap(); // 35% prob
        std::io::repeat(C_BYTE).take(20).read_to_end(&mut buffer).unwrap(); // 20% prob

        buffer
    }

    #[test]
    fn test_statistic_fns() {
        let test_data = get_test_data();

        let counts = count_simple(&test_data);
        assert_eq!(counts[A_BYTE as usize], 45);
        assert_eq!(counts[B_BYTE as usize], 35);
        assert_eq!(counts[C_BYTE as usize], 20);

        assert_eq!(get_num_unique_symbols(&counts), 3);

        let sorted_counts = get_sorted_symbols(&counts);
        assert_eq!(sorted_counts[0].symbol, A_BYTE);

        let max_val = get_table_max_val(&counts);
        assert_eq!(max_val, 31);

        let column_heigths = get_column_heights(&counts);
        assert_eq!(column_heigths, &[14, 10, 6]);
        // assert_eq!(get_column_height(&counts, A_BYTE), 14);
        // assert_eq!(get_column_height(&counts, B_BYTE), 10);
        // assert_eq!(get_column_height(&counts, C_BYTE), 6);
    }

    #[test]
    fn test_create_table() {
        let test_data = get_test_data();
        let counts = count_simple(&test_data);
        assert_eq!(counts[A_BYTE as usize], 45);
        assert_eq!(counts[B_BYTE as usize], 35);
        assert_eq!(counts[C_BYTE as usize], 20);
    }
}
