use core::cmp::Ordering;

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Node {
    /// the symbol, limited to single byte alphabet
    pub(crate) symbol: Option<u8>,
    /// the number of occurences
    pub(crate) count: u32,
    /// position of the left node in the array
    pub(crate) left: Option<u16>,
    /// position of the right node in the array
    pub(crate) right: Option<u16>,
    pub(crate) number_bits: u8,
    // /// position of the üarent node in the array
    // parent: Option<u16>,
}

impl std::cmp::PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.count.cmp(&self.count))
    }
}

// The priority queue depends on `Ord`.
// Explicitly implement the trait so the queue becomes a min-heap
// instead of a max-heap.
impl std::cmp::Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that the we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of `PartialEq` and `Ord` consistent.
        other.count.cmp(&self.count)
    }
}

impl core::fmt::Debug for Node {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "Node{{ symbol:{:?} count:{} number_bits:{:?} }}",
            self.symbol, self.count, self.number_bits
        ))
    }
}

#[derive(Default, Clone, Copy)]
pub struct MinNode {
    pub(crate) number_bits: u8,
    pub(crate) val: u16,
}

impl core::fmt::Debug for MinNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // let
        f.write_fmt(format_args!(
            "Node{{ val:{:#08b} number_bits:{:?} }}",
            self.val, self.number_bits
        ))
    }
}
