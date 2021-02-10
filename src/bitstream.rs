/// returns the position of the highest bit
///
/// see test_bit_highbit32
#[inline]
pub fn bit_highbit32(val: u32) -> u32 {
    return val.leading_zeros() ^ 31;
}

#[test]
fn test_bit_highbit32() {
    assert_eq!(bit_highbit32(1), 0);
    assert_eq!(bit_highbit32(2), 1);
    assert_eq!(bit_highbit32(4), 2);
    assert_eq!(bit_highbit32(7), 2);
    assert_eq!(bit_highbit32(8), 3);
    assert_eq!(bit_highbit32(9), 3);
    assert_eq!(bit_highbit32(1000), 9);
    assert_eq!(bit_highbit32(1024), 10);
}

const BIT_MASK: [u32; 32] = [
    0u32,
    1u32,
    3u32,
    7u32,
    0xfu32,
    0x1fu32,
    0x3fu32,
    0x7fu32,
    0xffu32,
    0x1ffu32,
    0x3ffu32,
    0x7ffu32,
    0xfffu32,
    0x1fffu32,
    0x3fffu32,
    0x7fffu32,
    0xffffu32,
    0x1ffffu32,
    0x3ffffu32,
    0x7ffffu32,
    0xfffffu32,
    0x1fffffu32,
    0x3fffffu32,
    0x7fffffu32,
    0xffffffu32,
    0x1ffffffu32,
    0x3ffffffu32,
    0x7ffffffu32,
    0xfffffffu32,
    0x1fffffffu32,
    0x3fffffffu32,
    0x7fffffffu32,
];

/// bitStream can mix input from multiple sources.
/// A critical property of these streams is that they encode and decode in **reverse** direction.
/// So the first bit sequence you add will be the last to be read, like a LIFO stack.
///
#[derive(Debug)]
pub struct BitCstream {
    pub(crate) bit_container: usize,
    pub(crate) bit_pos: u32,
    pub(crate) data_pos: usize,
    pub data: Vec<u8>,
}

impl BitCstream {
    pub(crate) fn new(capacity: usize) -> Self {
        let mut data: Vec<u8> = Vec::new();
        data.resize(capacity, 0);
        BitCstream {
            bit_container: 0,
            bit_pos: 0,
            data_pos: 0,
            data,
        }
    }
}

impl BitCstream {
    /// can add up to 31 bits into `bitC`.
    /// Note : does not check for register overflow !
    #[inline]
    pub fn add_bits(&mut self, value: usize, nb_bits: u32) {
        // debug_assert!(nbBits < BIT_MASK_SIZE);

        self.bit_container |= (value & BIT_MASK[nb_bits as usize] as usize) << self.bit_pos;
        self.bit_pos += nb_bits;
    }

    /// works only if `value` is _clean, meaning all high bits above nb_bits are 0
    #[inline]
    pub fn add_bits_fast(&mut self, value: usize, nb_bits: u32) {
        // debug_assert!(nb_bits < BIT_MASK_SIZE);

        self.bit_container |= value << self.bit_pos;
        self.bit_pos += nb_bits;
    }

    /// assumption : bitContainer has not overflowed
    /// unsafe version; does not check buffer overflow */
    #[inline]
    pub fn flush_bits_fast(&mut self) {
        let nb_bytes = self.bit_pos >> 3;

        debug_assert!(self.bit_pos < core::mem::size_of_val(&self.bit_container) as u32 * 8);

        self.data
            .extend_from_slice(&self.bit_container.to_le_bytes());

        debug_assert!(self.data.len() > self.data_pos);
        push_usize(&mut self.data, self.data_pos, self.bit_container);

        self.data_pos += nb_bytes as usize;
        self.bit_pos &= 7;
        self.bit_container >>= nb_bytes * 8;
    }

    /// assumption : bitContainer has not overflowed
    /// unsafe version; does not check buffer overflow */
    #[inline]
    pub fn finish_stream(&mut self) {
        self.add_bits_fast(1, 1);
        self.flush_bits_fast();
    }
}

#[inline]
fn push_usize(output: &mut Vec<u8>, pos: usize, el: usize) {
    unsafe {
        let out_ptr = output.as_mut_ptr().add(pos);
        core::ptr::copy_nonoverlapping(
            el.to_le_bytes().as_ptr(),
            out_ptr,
            core::mem::size_of::<usize>(),
        );
    }
}
