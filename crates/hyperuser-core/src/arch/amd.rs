use crate::arch::{Hypervisor, HvError};
use crate::cpu;
use hyperuser_proto::registers::GuestRegisters;

pub struct AmdSvm;

impl Hypervisor for AmdSvm {
    fn is_supported() -> bool {
        cpu::is_svm_supported()
    }
    
    fn enable() -> Result<(), HvError> {
        // Check if SVM is disabled by BIOS (MSR VM_CR bit 4)
        unsafe {
            let vm_cr = cpu::rdmsr(cpu::MSR_VM_CR);
            if vm_cr & (1 << 4) != 0 {
                return Err(HvError::FeatureControlLocked);
            }
            // Enable SVM by setting EFER.SVME (bit 12)
            let efer = cpu::rdmsr(cpu::IA32_EFER);
            cpu::wrmsr(cpu::IA32_EFER, efer | (1 << 12));
        }
        log::info!("[AMD] SVM enabled via EFER.SVME");
        Ok(())
    }
    
    fn initialize() -> Result<(), HvError> {
        // TODO: Phase 2
        // 1. Allocate Host Save Area (page-aligned)
        // 2. Write HSA physical address to MSR_VM_HSAVE_PA
        // 3. Allocate VMCB (4KB aligned)
        // 4. Initialize VMCB control area and state save area
        // 5. Build Nested Page Tables (NPT)
        log::info!("[AMD] SVM initialization placeholder");
        Ok(())
    }
    
    fn launch(guest_regs: &mut GuestRegisters) -> Result<(), HvError> {
        // TODO: Phase 2
        // 1. Load guest state into VMCB
        // 2. VMRUN
        // 3. On #VMEXIT: save guest state, dispatch handler
        let _ = guest_regs;
        log::info!("[AMD] VM launch placeholder");
        Ok(())
    }
}
