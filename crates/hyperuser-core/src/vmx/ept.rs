use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EptEntry: u64 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2;
        
        const MEMORY_TYPE_UC = 0 << 3; // Uncacheable
        const MEMORY_TYPE_WC = 1 << 3;
        const MEMORY_TYPE_WT = 4 << 3;
        const MEMORY_TYPE_WP = 5 << 3;
        const MEMORY_TYPE_WB = 6 << 3; // Write-back
        
        const IGNORE_PAT = 1 << 6;
        const LARGE_PAGE = 1 << 7; // 2MB or 1GB page
        const ACCESSED = 1 << 8;
        const DIRTY = 1 << 9;
        const USER_EXECUTE = 1 << 10;
        
        const READ_WRITE_EXECUTE = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }
}

impl EptEntry {
    /// Extract physical page frame number (bits 12..=51)
    #[inline]
    pub fn address(&self) -> u64 {
        self.bits() & 0x000F_FFFF_FFFF_F000
    }

    /// Set bits 12..=51
    #[inline]
    pub fn set_address(&mut self, addr: u64) {
        let current = self.bits() & !0x000F_FFFF_FFFF_F000;
        *self = Self::from_bits_retain(current | (addr & 0x000F_FFFF_FFFF_F000));
    }

    /// Combine address and flags into raw u64
    #[inline]
    pub fn with_addr(addr: u64, flags: EptEntry) -> u64 {
        (addr & 0x000F_FFFF_FFFF_F000) | flags.bits()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EptPointer(u64);

impl EptPointer {
    pub fn new(pml4_pa: u64) -> Self {
        // Memory type WB (6), Page walk length 4 (3), PML4 PA
        let eptp = 6 | (3 << 3) | (pml4_pa & 0x000F_FFFF_FFFF_F000);
        Self(eptp)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Builds an identity mapped EPT (mapping Guest Physical to Host Physical 1:1)
/// Uses 2MB large pages.
/// Returns (PML4 physical address, EPTP value)
pub fn build_identity_ept(max_phys_addr: u64) -> (u64, u64) {
    // TODO: allocate pages from frame allocator and fill entries.
    // For now, this is a skeleton returning dummy values.
    //
    // 1. Allocate PML4
    // 2. Allocate PDPTs
    // 3. Allocate PDs (for 2MB pages)
    // 4. Fill PD entries with `EptEntry::READ_WRITE_EXECUTE | EptEntry::LARGE_PAGE | EptEntry::MEMORY_TYPE_WB`
    
    let pml4_pa = 0; // Placeholder
    let eptp = EptPointer::new(pml4_pa).as_u64();
    (pml4_pa, eptp)
}

/// Splits a 2MB page into 512 4KB entries
pub fn split_large_page(pde_index: usize, original_2mb_pa: u64) -> u64 {
    // TODO: implement page splitting
    // 1. Allocate new PT (Page Table)
    // 2. Fill 512 entries with 4KB chunks of the original 2MB block
    // 3. Update the PD entry to point to this new PT without LARGE_PAGE flag
    // 4. Return physical address of new PT
    0 // Placeholder
}
