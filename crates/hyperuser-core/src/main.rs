#![no_std]
#![no_main]

extern crate alloc;

mod cpu;
mod arch;
mod vmx;
mod paging;
mod mm;
mod handler;
mod hook;

use uefi::prelude::*;

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to initialize UEFI helpers");
    
    log::info!("[*] HyperUser Type-1 Hypervisor v{}", env!("CARGO_PKG_VERSION"));
    log::info!("[*] Detecting CPU vendor...");
    
    let vendor = cpu::detect_vendor();
    log::info!("[*] CPU Vendor: {:?}", vendor);
    
    match vendor {
        hyperuser_proto::CpuVendor::Intel => {
            log::info!("[*] Intel VT-x detected");
            if !cpu::is_vmx_supported() {
                log::error!("[!] VMX not supported on this CPU");
                return Status::UNSUPPORTED;
            }
            log::info!("[+] VMX is supported");
        }
        hyperuser_proto::CpuVendor::Amd => {
            log::info!("[*] AMD-V detected");
            if !cpu::is_svm_supported() {
                log::error!("[!] SVM not supported on this CPU");
                return Status::UNSUPPORTED;
            }
            log::info!("[+] SVM is supported");
        }
        hyperuser_proto::CpuVendor::Unknown => {
            log::error!("[!] Unknown CPU vendor — cannot enable virtualization");
            return Status::UNSUPPORTED;
        }
    }
    
    // Allocate 16MB of RuntimeServicesData memory for hypervisor use
    unsafe {
        if let Some(st) = uefi::helpers::system_table().as_mut() {
            // Memory allocation requires boot services
            // We'll just cast the pointer and call AllocatePages
            let bs = st.boot_services() as *const _ as *mut hook::EfiBootServices;
            let mut addr = 0u64;
            // AllocateAnyPages = 0, EfiRuntimeServicesData = 6
            let allocate_pages: extern "efiapi" fn(u32, u32, usize, *mut u64) -> uefi::Status = 
                core::mem::transmute((*bs).allocate_pages);
            
            // 16MB = 4096 pages (4KB each)
            let status = allocate_pages(0, 6, 4096, &mut addr);
            if status == uefi::Status::SUCCESS {
                log::info!("[+] Allocated 16MB hypervisor memory at {:#x}", addr);
                mm::init_allocator(addr, 16 * 1024 * 1024);
            } else {
                log::error!("[!] Failed to allocate hypervisor memory");
                return Status::ABORTED;
            }
        }
    }
    
    log::info!("[*] HyperUser initialization complete. Waiting for ExitBootServices...");
    
    // Install ExitBootServices hook and chainload OS bootloader
    unsafe {
        if let Some(st) = uefi::helpers::system_table().as_mut() {
            let bs_ptr = st.boot_services() as *const _ as *mut core::ffi::c_void;
            hook::install_ebs_hook(bs_ptr);
        } else {
            log::error!("[!] Failed to get system table");
            return Status::ABORTED;
        }
    }
    
    // For now, just return success (the OS loader will be launched by the firmware)
    Status::SUCCESS
}
