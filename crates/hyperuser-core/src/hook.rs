//! ExitBootServices hook for transparent hypervisor installation.
//!
//! # Architecture
//!
//! 1. Before the OS bootloader is chainloaded, we patch the UEFI Boot Services
//!    table to replace `ExitBootServices` with our hook function.
//! 2. When the OS loader calls `ExitBootServices`, our hook fires.
//! 3. The hook calls the original `ExitBootServices` to tear down boot services.
//! 4. After EBS returns successfully, we:
//!    a. Enable VMX on the BSP (Bootstrap Processor)
//!    b. Set up VMCS with guest state pointing to the return address of EBS
//!    c. Execute VMLAUNCH — the OS resumes as a guest VM
//! 5. The OS is now running inside our hypervisor, completely unaware.
//!
//! # Important Notes
//!
//! - After modifying the `exit_boot_services` pointer in the Boot Services table,
//!   the table header's CRC32 **must** be recalculated. Windows `bootmgfw.efi`
//!   validates this checksum.
//! - All hypervisor memory (VMCS, EPT, stacks) must be allocated as
//!   `EfiRuntimeServicesData` or `EfiReservedMemoryType` so the OS kernel
//!   doesn't reclaim it after ExitBootServices.

use core::sync::atomic::{AtomicU64, Ordering};

/// Saved original ExitBootServices function pointer (as a raw u64).
static ORIGINAL_EBS: AtomicU64 = AtomicU64::new(0);

/// Type alias for the ExitBootServices function signature.
type ExitBootServicesFn = unsafe extern "efiapi" fn(
    image_handle: uefi::Handle,
    map_key: usize,
) -> uefi::Status;

#[repr(C)]
pub struct EfiTableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct EfiBootServices {
    pub hdr: EfiTableHeader,
    pub raise_tpl: usize,
    pub restore_tpl: usize,
    pub allocate_pages: usize,
    pub free_pages: usize,
    pub get_memory_map: usize,
    pub allocate_pool: usize,
    pub free_pool: usize,
    pub create_event: usize,
    pub set_timer: usize,
    pub wait_for_event: usize,
    pub signal_event: usize,
    pub close_event: usize,
    pub check_event: usize,
    pub install_protocol_interface: usize,
    pub reinstall_protocol_interface: usize,
    pub uninstall_protocol_interface: usize,
    pub handle_protocol: usize,
    pub reserved: usize,
    pub register_protocol_notify: usize,
    pub locate_handle: usize,
    pub locate_device_path: usize,
    pub install_configuration_table: usize,
    pub load_image: usize,
    pub start_image: usize,
    pub exit: usize,
    pub unload_image: usize,
    pub exit_boot_services: usize,
}

fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFFFFFFu32
}

/// Install the ExitBootServices hook into the UEFI Boot Services table.
///
/// # Safety
///
/// - Must be called while Boot Services are still active.
/// - Must recalculate the CRC32 of the Boot Services table header after modification.
/// - The hypervisor memory regions must already be allocated as RuntimeServices memory.
pub unsafe fn install_ebs_hook(bs_ptr: *mut core::ffi::c_void) {
    let bs = bs_ptr as *mut EfiBootServices;

    // Save original exit_boot_services function pointer
    let original = (*bs).exit_boot_services;
    ORIGINAL_EBS.store(original as u64, Ordering::SeqCst);

    // Replace with our hook
    (*bs).exit_boot_services = _hooked_exit_boot_services as usize;

    // Recalculate CRC32 of the Boot Services table header
    (*bs).hdr.crc32 = 0;
    let header_size = (*bs).hdr.header_size as usize;
    let bs_bytes = core::slice::from_raw_parts(bs_ptr as *const u8, header_size);
    let new_crc = compute_crc32(bs_bytes);
    (*bs).hdr.crc32 = new_crc;

    log::info!("[*] ExitBootServices hook installed successfully");
}

/// Our hooked ExitBootServices function.
///
/// When the OS bootloader calls `ExitBootServices`, this function intercepts
/// the call. After the original EBS succeeds (boot services are torn down),
/// we initialize the hypervisor and launch the OS as a guest VM.
///
/// # Safety
///
/// Called by the OS bootloader when it's ready to take control of the machine.
/// At this point, boot services are being shut down. After the original EBS
/// returns `SUCCESS`, we must not use any boot service APIs.
unsafe extern "efiapi" fn _hooked_exit_boot_services(
    image_handle: uefi::Handle,
    map_key: usize,
) -> uefi::Status {
    // Retrieve the original ExitBootServices pointer
    let original_ptr = ORIGINAL_EBS.load(Ordering::SeqCst);
    if original_ptr == 0 {
        return uefi::Status::DEVICE_ERROR;
    }
    let original: ExitBootServicesFn = core::mem::transmute(original_ptr);

    // Call the original ExitBootServices
    let status = original(image_handle, map_key);
    if status != uefi::Status::SUCCESS {
        // Memory map key was stale — return error so the OS bootloader
        // retries with GetMemoryMap + ExitBootServices.
        return status;
    }

    // ═══════════════════════════════════════════════════════════════
    // POST-ExitBootServices — Boot services are DEAD.
    // CPU is in long mode with identity mapping. No allocator, no UEFI APIs.
    // ═══════════════════════════════════════════════════════════════

    use crate::arch::Hypervisor;
    
    // For now, we assume Intel VT-x
    let is_intel = crate::arch::intel::IntelVmx::is_supported();
    if is_intel {
        if let Err(_) = crate::arch::intel::IntelVmx::enable() {
            return uefi::Status::ABORTED;
        }
        
        if let Err(_) = crate::arch::intel::IntelVmx::initialize() {
            return uefi::Status::ABORTED;
        }

        // Dummy guest registers for now. In reality, we need an assembly stub
        // to capture the precise GPRs at this exact moment and load them.
        let mut guest_regs = hyperuser_proto::registers::GuestRegisters::default();
        
        if let Err(_) = crate::arch::intel::IntelVmx::launch(&mut guest_regs) {
            return uefi::Status::ABORTED;
        }
    }

    uefi::Status::SUCCESS
}
