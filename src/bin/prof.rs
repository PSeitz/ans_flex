use ans_flex::compress;
use ans_flex::decompress;
use ans_flex::table::fse_optimal_table_log;
use ans_flex::FSE_DEFAULT_TABLELOG;
use common::count_simple;
use common::get_max_symbol_value;
use common::get_normalized_counts;

// fn main() {
//     const COMPRESSION66K: &'static [u8] = include_bytes!("../../benches/compression_66k_JSON.txt");
//     let mut len = 0;
//     // for _ in 0..1 {
//     let yo = compress(COMPRESSION66K);
//     len += yo.data_pos;
//     // }
//     println!("yo.data_pos {:?}", yo.data_pos);
//     println!("COMPRESSION66K {:?}", COMPRESSION66K.len());
// }

fn main() {
    const COMPRESSION66K: &'static [u8] =
        include_bytes!("../../test_data/compression_66k_JSON.txt");
    let test_data = COMPRESSION66K;
    let mut len = 0;
    // let yo = compress(COMPRESSION66K);
    let counts = count_simple(&test_data);
    let out = compress(&test_data);
    let max_symbol_value = get_max_symbol_value(&counts);
    let table_log = fse_optimal_table_log(FSE_DEFAULT_TABLELOG, test_data.len(), max_symbol_value);
    let norm_counts = get_normalized_counts(&counts, table_log, test_data.len(), max_symbol_value);
    for _ in 0..10 {
        // dbg!(&out.get_compressed_data());
        // dbg!("out.get_compressed_data().len() {:?}", out.get_compressed_data().len());
        let decompressed = decompress(
            &out.get_compressed_data(),
            &norm_counts,
            table_log,
            test_data.len(),
            max_symbol_value,
        );

        len += decompressed.len();
    }
    println!("len {:?}", len);
    println!("COMPRESSION66K {:?}", COMPRESSION66K.len());
}
