/// A decomposed 64-bit virtual memory pointer.
/// Note the top 16 bits of a 64-bit virtual memory address are generally ignored when using
/// four-level paging.
pub struct DecomposedPointer {
    /// 9 bit index into lvl 4 table.
    pub pml4_index: u16,
    /// 9 bits index into lvl 3 table
    pub directory_ptr_index: u16,
    /// 9 bit index into lvl 2 table
    pub directory_index: u16,
    /// 9 bit index into lvl 1 table
    pub table_index: u16,
    /// 12, 21, or 30 bit (based on system page size) byte offset into page
    pub page_offset: u32,
}

impl DecomposedPointer {
    /// Decompose a virtual memory address to its constituent page table indices and page byte
    /// offset. Currently assumes 4k pages, so the page offset will never be larger than 12 bits.
    /// That is to say, this will not work properly for 2mb or 1gb pages.
    #[allow(clippy::unusual_byte_groupings)]
    pub fn new(virtual_address: u64) -> Self {
        let pml4_index = ((virtual_address >> 39) & 0x1ff) as u16;
        let directory_ptr_index = ((virtual_address >> 30) & 0x1ff) as u16;
        let directory_index = ((virtual_address >> 21) & 0x1ff) as u16;
        let table_index = ((virtual_address >> 12) & 0x1ff) as u16;
        let page_offset = (virtual_address & 0xfff) as u32;

        Self {
            pml4_index,
            directory_ptr_index,
            directory_index,
            table_index,
            page_offset,
        }
    }
}

impl std::fmt::Binary for DecomposedPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:011b} | {:011b} | {:011b} | {:011b} | {:013b}",
            self.pml4_index,
            self.directory_ptr_index,
            self.directory_index,
            self.table_index,
            self.page_offset,
        )
    }
}

impl std::fmt::Display for DecomposedPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "pml4 idx: {:#05x}, dir ptr idx: {:#05x}, dir idx: {:#05x}, table idx: {:#05x}, page offs: {:#07x}",
            self.pml4_index,
            self.directory_ptr_index,
            self.directory_index,
            self.table_index,
            self.page_offset,
        )
    }
}
