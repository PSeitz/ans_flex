pub(crate) type BitContainer = usize;
pub(crate) const BIT_CONTAINER_SIZE: usize = core::mem::size_of::<BitContainer>();

const REG_MASK: u32 = BIT_CONTAINER_SIZE as u32 * 8 - 1;

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

#[derive(Debug)]
pub enum BitDstreamStatus {
    Unfinished,
    EndOfBuffer,
    Completed,
    Overflow,
}

#[derive(Debug)]
pub struct BitDstream {
    pub(crate) bit_container: BitContainer,
    pub(crate) bits_consumed: u32,
    pub(crate) limit_pos: usize,
    pub(crate) input_pos: usize,
    // pub data: Vec<u8>,
}

impl BitDstream {
    pub fn new(input: &[u8]) -> Self {
        let limit_pos = BIT_CONTAINER_SIZE;

        if input.len() >= BIT_CONTAINER_SIZE {
            let input_pos = input.len() - BIT_CONTAINER_SIZE;
            let bit_container = read_usize(input, input_pos);
            let last_byte = input[input.len() - 1];

            let bits_consumed = if last_byte == 0 {
                // ensures bits_consumed is always set
                0
            } else {
                bit_highbit32(last_byte as u32)
            };

            BitDstream {
                bit_container,
                bits_consumed,
                limit_pos,
                input_pos,
            }
        } else {
            unimplemented!();
        }
    }

    /// On 32-bits, maxNbBits==24.
    /// On 64-bits, maxNbBits==56.
    fn look_bits(&self, nb_bits: u32) -> usize {
        debug_assert!(nb_bits >= 1);
        let start = BIT_CONTAINER_SIZE as u32 * 8 - self.bits_consumed - nb_bits;
        get_middle_bits(self.bit_container, start, nb_bits)
    }
    /// On 32-bits, maxNbBits==24.
    /// On 64-bits, maxNbBits==56.
    pub fn read_bits(&mut self, nb_bits: u32) -> usize {
        let value = self.look_bits(nb_bits);
        self.skip_bits(nb_bits);
        value
    }
    /// only words when nb_bits > 1.
    pub fn read_bits_fast(&mut self, nb_bits: u32) -> usize {
        debug_assert!(nb_bits >= 1);
        let value = self.look_bits_fast(nb_bits);
        self.skip_bits(nb_bits);
        value
    }

    /// only words when nb_bits > 1
    fn look_bits_fast(&self, nb_bits: u32) -> usize {
        debug_assert!(nb_bits >= 1);
        (self.bit_container << (self.bits_consumed & REG_MASK))
            >> (((REG_MASK + 1) - nb_bits) & REG_MASK)
    }

    fn skip_bits(&mut self, nb_bits: u32) {
        self.bits_consumed += nb_bits;
    }
}

fn get_middle_bits(bit_container: usize, start: u32, nb_bits: u32) -> usize {
    debug_assert!(nb_bits < BIT_MASK.len() as u32);
    (bit_container >> (start & REG_MASK) as usize) & BIT_MASK[nb_bits as usize] as usize
}

/// bitStream can mix input from multiple sources.
/// A critical property of these streams is that they encode and decode in **reverse** direction.
/// So the first bit sequence you add will be the last to be read, like a LIFO stack.
///
#[derive(Debug)]
pub struct BitCstream {
    pub(crate) bit_container: BitContainer,
    pub(crate) bit_pos: u32,
    pub data_pos: usize,
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
        // debug_assert!(nb_bits < BIT_MASK_SIZE);

        // unsafe here adds around 0-7% performance gains
        let bit_mask = unsafe { BIT_MASK.get_unchecked(nb_bits as usize) };
        self.bit_container |= (value & *bit_mask as usize) << self.bit_pos;

        // self.bit_container |= (value & BIT_MASK[nb_bits as usize] as usize) << self.bit_pos;
        self.bit_pos += nb_bits;
    }

    /// works only if `value` is _clean, meaning all high bits above nb_bits are 0
    #[inline]
    pub fn add_bits_fast(&mut self, value: usize, nb_bits: u32) {
        // debug_assert!(nb_bits < BIT_MASK_SIZE);

        self.bit_container |= value << self.bit_pos;
        self.bit_pos += nb_bits;
    }

    /// assumption : bit_container has not overflowed
    /// unsafe version; does not check buffer overflow */
    #[inline]
    pub fn flush_bits_fast(&mut self) {
        let nb_bytes = self.bit_pos >> 3;

        debug_assert!(self.bit_pos < core::mem::size_of_val(&self.bit_container) as u32 * 8);

        // self.data
        //     .extend_from_slice(&self.bit_container.to_le_bytes());

        debug_assert!(self.data.len() > self.data_pos);
        push_usize(&mut self.data, self.data_pos, self.bit_container);

        self.data_pos += nb_bytes as usize;
        self.bit_pos &= 7;
        self.bit_container >>= nb_bytes * 8;
    }

    /// assumption: bit_container has not overflowed
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

#[inline]
fn read_usize(input: &[u8], pos: usize) -> usize {
    let mut num: usize = 0;
    unsafe {
        core::ptr::copy_nonoverlapping(
            input.as_ptr().add(pos),
            &mut num as *mut usize as *mut u8,
            core::mem::size_of::<usize>(),
        );
    }
    num
}
