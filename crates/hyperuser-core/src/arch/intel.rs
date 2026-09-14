use crate::arch::{Hypervisor, HvError};
use crate::cpu;
use hyperuser_proto::registers::GuestRegisters;

pub struct IntelVmx;

impl Hypervisor for IntelVmx {
    fn is_supported() -> bool {
        cpu::is_vmx_supported()
    }
    
    fn enable() -> Result<(), HvError> {
        unsafe { cpu::enable_vmx().map_err(|_| HvError::FeatureControlLocked) }
    }
    
    fn initialize() -> Result<(), HvError> {
        log::info!("[Intel] Starting VMX initialization...");
        
        unsafe {
            // 1. Get VMX revision identifier
            let vmx_basic = cpu::rdmsr(cpu::IA32_VMX_BASIC);
            let revision_id = vmx_basic as u32;

            // 2. Allocate VMXON region (4KB aligned)
            let vmxon_region = crate::mm::alloc_frame().ok_or(HvError::OutOfMemory)?;
            core::ptr::write(vmxon_region.as_u64() as *mut u32, revision_id);
            
            // 3. Execute VMXON
            crate::vmx::vmxon(vmxon_region.as_u64()).map_err(|_| HvError::VmInstructionFailed)?;
            log::info!("[Intel] VMXON successful");

            // 4. Allocate VMCS region
            let vmcs_region = crate::mm::alloc_frame().ok_or(HvError::OutOfMemory)?;
            core::ptr::write(vmcs_region.as_u64() as *mut u32, revision_id);

            // 5. VMCLEAR
            crate::vmx::vmclear(vmcs_region.as_u64()).map_err(|_| HvError::VmInstructionFailed)?;
            
            // 6. VMPTRLD
            crate::vmx::vmptrld(vmcs_region.as_u64()).map_err(|_| HvError::VmInstructionFailed)?;
            log::info!("[Intel] VMCS loaded successfully");

            // 7. Initialize VMCS fields
            // TODO: Call into crate::vmx::vmcs to setup fields
        }
        
        Ok(())
    }
    
    fn launch(guest_regs: &mut GuestRegisters) -> Result<(), HvError> {
        log::info!("[Intel] Executing VMLAUNCH...");
        
        unsafe {
            // Write guest GPRs to VMCS or just execute launch
            // Actually GPRs are usually restored right before VMLAUNCH via assembly
            // For now, we assume the assembly stub handles GPR restoration
            
            crate::vmx::vmlaunch().map_err(|_| HvError::VmInstructionFailed)?;
        }
        
        // If VMLAUNCH returns, it failed
        log::error!("[Intel] VMLAUNCH returned unexpectedly!");
        Err(HvError::VmInstructionFailed)
    }
}
