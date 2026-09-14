use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageTableEntry: u64 {
        const PRESENT = 1 << 0;
        const WRITABLE = 1 << 1;
        const USER = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const LARGE_PAGE = 1 << 7;
        const GLOBAL = 1 << 8;
        const NO_EXECUTE = 1 << 63;
    }
}

impl PageTableEntry {
    /// Extract physical page frame number (bits 12..=51)
    #[inline]
    pub fn address(&self) -> u64 {
        self.bits() & 0x000F_FFFF_FFFF_F000
    }
}

/// Translates a Guest Virtual Address to a Guest Physical Address using the Guest CR3.
/// NOTE: This walker currently reads from host virtual memory assuming an identity map.
pub fn translate_guest_va(cr3: u64, va: u64) -> Option<u64> {
    let pml4_base = cr3 & 0x000F_FFFF_FFFF_F000;
    
    let pml4_index = ((va >> 39) & 0x1FF) as usize;
    let pdpt_index = ((va >> 30) & 0x1FF) as usize;
    let pd_index = ((va >> 21) & 0x1FF) as usize;
    let pt_index = ((va >> 12) & 0x1FF) as usize;

    unsafe {
        // Walk PML4
        let pml4 = core::slice::from_raw_parts(pml4_base as *const u64, 512);
        let pml4e = PageTableEntry::from_bits_truncate(pml4[pml4_index]);
        if !pml4e.contains(PageTableEntry::PRESENT) {
            return None;
        }

        // Walk PDPT
        let pdpt_base = pml4e.address();
        let pdpt = core::slice::from_raw_parts(pdpt_base as *const u64, 512);
        let pdpte = PageTableEntry::from_bits_truncate(pdpt[pdpt_index]);
        if !pdpte.contains(PageTableEntry::PRESENT) {
            return None;
        }
        if pdpte.contains(PageTableEntry::LARGE_PAGE) {
            // 1GB page
            let page_offset = va & ((1 << 30) - 1);
            return Some((pdpte.address() & !((1 << 30) - 1)) + page_offset);
        }

        // Walk PD
        let pd_base = pdpte.address();
        let pd = core::slice::from_raw_parts(pd_base as *const u64, 512);
        let pde = PageTableEntry::from_bits_truncate(pd[pd_index]);
        if !pde.contains(PageTableEntry::PRESENT) {
            return None;
        }
        if pde.contains(PageTableEntry::LARGE_PAGE) {
            // 2MB page
            let page_offset = va & ((1 << 21) - 1);
            return Some((pde.address() & !((1 << 21) - 1)) + page_offset);
        }

        // Walk PT
        let pt_base = pde.address();
        let pt = core::slice::from_raw_parts(pt_base as *const u64, 512);
        let pte = PageTableEntry::from_bits_truncate(pt[pt_index]);
        if !pte.contains(PageTableEntry::PRESENT) {
            return None;
        }

        // 4KB page
        let page_offset = va & ((1 << 12) - 1);
        Some(pte.address() + page_offset)
    }
}
