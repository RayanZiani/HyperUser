//! Memory utilities for aligned allocation and virtual-to-physical
//! address translation on Windows.
//!
//! Provides helpers for allocating page-aligned memory (required for
//! hypercall packets) and querying process memory layout.

#[cfg(windows)]
use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};

/// Allocate page-aligned, committed memory.
///
/// Uses `VirtualAlloc` on Windows to guarantee the returned pointer
/// is aligned to the system page size (typically 4096 bytes), which
/// exceeds the 16-byte alignment requirement of [`HypercallPacket`].
///
/// # Returns
///
/// A pointer to the allocated region, or `None` if allocation failed.
#[cfg(windows)]
pub fn alloc_aligned(size: usize) -> Option<*mut u8> {
    unsafe {
        let ptr = VirtualAlloc(None, size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
        if ptr.is_null() {
            None
        } else {
            Some(ptr as *mut u8)
        }
    }
}

/// Free memory previously allocated with [`alloc_aligned`].
#[cfg(windows)]
pub fn free_aligned(ptr: *mut u8) {
    unsafe {
        let _ = VirtualFree(ptr as _, 0, MEM_RELEASE);
    }
}

/// Retrieve the CR3 (Directory Table Base) for a given process.
///
/// This requires the hypervisor to be active and capable of reading
/// the `KPROCESS.DirectoryTableBase` field from physical memory.
///
/// # Arguments
///
/// * `_pid` — Windows Process ID (PID) of the target process.
///
/// # Returns
///
/// The CR3 value, or `None` if retrieval failed.
pub fn get_process_cr3(_pid: u32) -> Option<u64> {
    // TODO: Phase 2
    // 1. Use NtQuerySystemInformation to enumerate EPROCESS addresses
    // 2. Use ReadPhys hypercall to read DirectoryTableBase from EPROCESS
    // 3. Return the CR3 value
    //
    // Offset of DirectoryTableBase in EPROCESS varies by Windows version:
    //   Windows 10 21H2+: ~0x028
    //   Windows 11 24H2:  ~0x028
    None
}

/// Translate a virtual address to a physical address using the hypervisor.
///
/// Sends a hypercall that instructs the hypervisor to walk the guest
/// page tables for the given CR3 and resolve the virtual address.
///
/// # Arguments
///
/// * `_cr3` — The CR3 value (page directory base) of the target process.
/// * `_va`  — The virtual address to translate.
///
/// # Returns
///
/// The physical address, or `None` if the page is not present / translation failed.
pub fn translate_va_to_pa(_cr3: u64, _va: u64) -> Option<u64> {
    // TODO: Phase 2
    // Use the hypervisor's page table walker via a dedicated hypercall
    // or by issuing multiple ReadPhys calls to walk PML4→PDPT→PD→PT
    None
}
