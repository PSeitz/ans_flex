/*! 

Bitstream handles the writing and reading of bits in an optimized manner.


Some bit hacks are appplied here, it can be helpful to understand these
Bit Operations:

number of bits  >> 3 == number of bytes

*/

pub(crate) type BitContainer = usize;
pub(crate) const BIT_CONTAINER_BYTES: usize = core::mem::size_of::<BitContainer>();
pub(crate) const NUM_BITS_IN_BIT_CONTAINER: u32 = BIT_CONTAINER_BYTES as u32 * 8;

const REG_MASK: u32 = NUM_BITS_IN_BIT_CONTAINER - 1;

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
    2_u32.pow(0)-1,
    2_u32.pow(1)-1,
    2_u32.pow(2)-1,
    2_u32.pow(3)-1,
    2_u32.pow(4)-1,
    2_u32.pow(5)-1,
    2_u32.pow(6)-1,
    2_u32.pow(7)-1,
    2_u32.pow(8)-1,
    2_u32.pow(9)-1,
    2_u32.pow(10)-1,
    2_u32.pow(11)-1,
    2_u32.pow(12)-1,
    2_u32.pow(13)-1,
    2_u32.pow(14)-1,
    2_u32.pow(15)-1,
    2_u32.pow(16)-1,
    2_u32.pow(17)-1,
    2_u32.pow(18)-1,
    2_u32.pow(19)-1,
    2_u32.pow(20)-1,
    2_u32.pow(21)-1,
    2_u32.pow(22)-1,
    2_u32.pow(23)-1,
    2_u32.pow(24)-1,
    2_u32.pow(25)-1,
    2_u32.pow(26)-1,
    2_u32.pow(27)-1,
    2_u32.pow(28)-1,
    2_u32.pow(29)-1,
    2_u32.pow(30)-1,
    2_u32.pow(31)-1,
];

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BitDstreamStatus {
    Unfinished,
    EndOfBuffer,
    Completed,
    Overflow,
}

#[derive(Debug)]
pub struct BitDstream {
    /// BitContainer is usize
    pub(crate) bit_container: BitContainer,

    /// Current number of bits consumed
    /// 
    /// Should be smaller than NUM_BITS_IN_BIT_CONTAINER
    pub(crate) bits_consumed: u32,

    /// slow read threshold for the last bytes
    /// after we pass this limit, we can't fast read bytes without checks
    pub(crate) limit_pos: usize,

    // starts at the end of the input
    pub(crate) input_pos: usize,
}

impl BitDstream {
    pub fn new(input: &[u8]) -> Self {
        let limit_pos = BIT_CONTAINER_BYTES;
        let (input_pos, bit_container) = if input.len() >= BIT_CONTAINER_BYTES {
            let input_pos = input.len() - BIT_CONTAINER_BYTES;
            let bit_container = read_usize(input, input_pos);
            (input_pos, bit_container)
        } else {
            let input_pos = 0;

            let mut bytes = [0,0,0,0,0,0,0,0];
            bytes[..input.len()].copy_from_slice(&input);
            let bit_container = usize::from_le_bytes(bytes);
            (input_pos, bit_container)
        };

        let last_byte = input[input.len() - 1];
        if last_byte == 0 {
            panic!("CORRUPTION DETECTED - END MARK NOT PRESENT"); // TODO proper error maybe
        }

        let mut bits_consumed = if last_byte == 0 {
            0
        } else {
            8 - bit_highbit32(last_byte as u32)
        };

        if input.len() < BIT_CONTAINER_BYTES {
            bits_consumed += (BIT_CONTAINER_BYTES as u32 - input.len() as u32) * 8;
        }

        BitDstream {
            bit_container,
            bits_consumed,
            limit_pos,
            input_pos,
        }
    }

    #[inline]
    pub fn reload_stream_fast(&mut self, input: &[u8]) -> BitDstreamStatus {
        // if (UNLIKELY(bitD->ptr < bitD->limitPtr))
        // return BIT_DStream_overflow;
        debug_assert!(self.bits_consumed <= NUM_BITS_IN_BIT_CONTAINER);
        self.input_pos -= self.bits_consumed as usize >> 3;
        self.bits_consumed &= 7;
        self.bit_container = read_usize(input, self.input_pos);
        BitDstreamStatus::Unfinished
    }
    #[inline]
    pub fn reload_stream(&mut self, input: &[u8]) -> BitDstreamStatus {
        if self.bits_consumed > NUM_BITS_IN_BIT_CONTAINER as u32 {
            return BitDstreamStatus::Overflow;
        }
        if self.input_pos >= self.limit_pos {
            return self.reload_stream_fast(input);
        }
        if self.input_pos == 0 {
            if self.bits_consumed < NUM_BITS_IN_BIT_CONTAINER {
                return BitDstreamStatus::EndOfBuffer;
            }
            return BitDstreamStatus::Completed;
        }
        // last 7 bytes
        let nb_bytes = self.bits_consumed >> 3;
        if  nb_bytes > self.input_pos as u32 {
            self.bits_consumed -= self.input_pos as u32 * 8;
            self.input_pos = 0;
            self.bit_container = read_usize(input, self.input_pos);
            BitDstreamStatus::EndOfBuffer
        }else{
            self.input_pos -= nb_bytes as usize;
            self.bits_consumed -= nb_bytes * 8;
            self.bit_container = read_usize(input, self.input_pos);
            BitDstreamStatus::Unfinished
        }
    }

    /// On 32-bits, maxNbBits==24.
    /// On 64-bits, maxNbBits==56.
    #[inline]
    fn look_bits(&self, nb_bits: u32) -> usize {
        debug_assert!(nb_bits >= 1);
        // debug_assert!(nb_bits <= 56);
        // dbg!(self.bits_consumed);
        // dbg!(nb_bits);
        // let start = NUM_BITS_IN_BIT_CONTAINER.wrapping_sub(self.bits_consumed).wrapping_sub(nb_bits);
        let start = NUM_BITS_IN_BIT_CONTAINER - self.bits_consumed - nb_bits;
        // if start > 1000 {
        //     dbg!(start);
        // }
        get_middle_bits(self.bit_container, start, nb_bits)
    }
    /// On 32-bits, maxNbBits==24.
    /// On 64-bits, maxNbBits==56.
    #[inline]
    pub fn read_bits(&mut self, nb_bits: u32) -> usize {
        // debug_assert!(nb_bits <= 56);
        // dbg!(nb_bits);
        let value = self.look_bits(nb_bits);

        self.bits_consumed += nb_bits;
        value
    }
    /// only words when nb_bits > 1.
    #[inline]
    pub fn read_bits_fast(&mut self, nb_bits: u32) -> usize {
        // dbg!(nb_bits);
        debug_assert!(nb_bits >= 1);
        let value = self.look_bits_fast(nb_bits);
        self.bits_consumed += nb_bits;
        value
    }

    /// only works when nb_bits > 1
    #[inline]
    fn look_bits_fast(&self, nb_bits: u32) -> usize {
        debug_assert!(nb_bits >= 1);

        // shifting left to shift out already consumed bits
        (self.bit_container << (self.bits_consumed & REG_MASK))
            >> ((NUM_BITS_IN_BIT_CONTAINER - nb_bits) & REG_MASK)
    }

}

#[inline]
fn get_middle_bits(bit_container: usize, start: u32, nb_bits: u32) -> usize {
    // dbg!((start & REG_MASK) as usize);
    // dbg!(bit_container >> (start & REG_MASK) as usize);
    (bit_container >> (start & REG_MASK) as usize) & *unsafe {BIT_MASK.get_unchecked(nb_bits as usize)} as usize
}

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
    #[inline]
    pub fn get_compressed_data(&self) -> &[u8] {
        &self.data[..self.get_compressed_size()]
    }

    #[inline]
    pub fn get_compressed_size(&self) -> usize {
        let last_byte = if self.bit_pos > 0 {
            1
        } else {
            0
        };
        self.data_pos + last_byte
    }

    /// can add up to 31 bits into `bitC`.
    /// Note : does not check for register overflow !
    #[inline]
    pub fn add_bits(&mut self, value: usize, nb_bits: u32) {
        debug_assert!(BIT_MASK.len() == 32);
        debug_assert!(nb_bits < BIT_MASK.len() as u32);

        // unsafe here adds around 0-7% performance gains
        let bit_mask = unsafe { BIT_MASK.get_unchecked(nb_bits as usize) };
        self.bit_container |= (value & *bit_mask as usize) << self.bit_pos;
        self.bit_pos += nb_bits;
    }

    /// works only if `value` is _clean, meaning all high bits above nb_bits are 0
    #[inline]
    pub fn add_bits_fast(&mut self, value: usize, nb_bits: u32) {
        debug_assert!(value >> nb_bits == 0);
        debug_assert!(nb_bits + self.bit_pos < NUM_BITS_IN_BIT_CONTAINER);

        self.bit_container |= value << self.bit_pos;
        self.bit_pos += nb_bits;
    }

    /// assumption : bit_container has not overflowed
    /// unsafe version; does not check buffer overflow */
    #[inline]
    pub fn flush_bits_fast(&mut self) {
        let nb_bytes = self.bit_pos >> 3;

        debug_assert!(self.bit_pos < NUM_BITS_IN_BIT_CONTAINER);

        // self.data
        //     .extend_from_slice(&self.bit_container.to_le_bytes());

        debug_assert!(self.data.len() > self.data_pos);

        // TODO check overflow for last bytes
        push_usize(&mut self.data, self.data_pos, self.bit_container); 

        self.data_pos += nb_bytes as usize;
        self.bit_pos &= 7;
        self.bit_container >>= nb_bytes * 8;
    }

    /// assumption: bit_container has not overflowed
    /// unsafe version; does not check buffer overflow
    #[inline]
    pub fn finish_stream(&mut self) {
        // add end mark
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
