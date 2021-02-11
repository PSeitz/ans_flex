use ans_flex::compress;

fn main() {
    const COMPRESSION66K: &'static [u8] = include_bytes!("../../benches/compression_66k_JSON.txt");
    let mut len = 0;
    // for _ in 0..1 {
    let yo = compress(COMPRESSION66K);
    len += yo.data_pos;
    // }
    println!("yo.data_pos {:?}", yo.data_pos);
    println!("COMPRESSION66K {:?}", COMPRESSION66K.len());
}
