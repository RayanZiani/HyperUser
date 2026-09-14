use bitflags::bitflags;

// Guest state fields
pub const GUEST_ES_SELECTOR: u32 = 0x0800;
pub const GUEST_CS_SELECTOR: u32 = 0x0802;
pub const GUEST_SS_SELECTOR: u32 = 0x0804;
pub const GUEST_DS_SELECTOR: u32 = 0x0806;
pub const GUEST_FS_SELECTOR: u32 = 0x0808;
pub const GUEST_GS_SELECTOR: u32 = 0x080A;
pub const GUEST_LDTR_SELECTOR: u32 = 0x080C;
pub const GUEST_TR_SELECTOR: u32 = 0x080E;
pub const GUEST_GDTR_BASE: u32 = 0x6816;
pub const GUEST_IDTR_BASE: u32 = 0x6818;
pub const GUEST_RFLAGS: u32 = 0x6820;
pub const GUEST_RIP: u32 = 0x681E;
pub const GUEST_RSP: u32 = 0x681C;
pub const GUEST_CR0: u32 = 0x6800;
pub const GUEST_CR3: u32 = 0x6802;
pub const GUEST_CR4: u32 = 0x6804;

// Host state fields
pub const HOST_ES_SELECTOR: u32 = 0x0C00;
pub const HOST_CS_SELECTOR: u32 = 0x0C02;
pub const HOST_SS_SELECTOR: u32 = 0x0C04;
pub const HOST_DS_SELECTOR: u32 = 0x0C06;
pub const HOST_FS_SELECTOR: u32 = 0x0C08;
pub const HOST_GS_SELECTOR: u32 = 0x0C0A;
pub const HOST_TR_SELECTOR: u32 = 0x0C0C;
pub const HOST_GDTR_BASE: u32 = 0x6C0C;
pub const HOST_IDTR_BASE: u32 = 0x6C0E;
pub const HOST_RIP: u32 = 0x6C16;
pub const HOST_RSP: u32 = 0x6C14;
pub const HOST_CR0: u32 = 0x6C00;
pub const HOST_CR3: u32 = 0x6C02;
pub const HOST_CR4: u32 = 0x6C04;

// Control fields
pub const PIN_BASED_VM_EXEC_CONTROLS: u32 = 0x4000;
pub const PRIMARY_PROC_BASED_VM_EXEC_CONTROLS: u32 = 0x4002;
pub const SECONDARY_PROC_BASED_VM_EXEC_CONTROLS: u32 = 0x401E;
pub const VM_EXIT_CONTROLS: u32 = 0x400C;
pub const VM_ENTRY_CONTROLS: u32 = 0x4012;
pub const EPT_POINTER: u32 = 0x201A;
pub const TSC_OFFSET: u32 = 0x2010;

// Additional guest/control fields
pub const GUEST_PHYSICAL_ADDRESS: u32 = 0x2400;
pub const MSR_BITMAP_ADDRESS: u32 = 0x2004;
pub const VPID: u32 = 0x0000;

// Exit info fields
pub const VM_EXIT_REASON: u32 = 0x4402;
pub const VM_EXIT_QUALIFICATION: u32 = 0x6400;
pub const VM_EXIT_INSTR_LEN: u32 = 0x440C;
pub const VM_EXIT_INSTR_INFO: u32 = 0x440E;
pub const VM_INSTRUCTION_ERROR: u32 = 0x4400;

// Exit reasons
pub const EXIT_REASON_CPUID: u32 = 10;
pub const EXIT_REASON_HLT: u32 = 12;
pub const EXIT_REASON_VMCALL: u32 = 18;
pub const EXIT_REASON_CR_ACCESS: u32 = 28;
pub const EXIT_REASON_MSR_READ: u32 = 31;
pub const EXIT_REASON_MSR_WRITE: u32 = 32;
pub const EXIT_REASON_EPT_VIOLATION: u32 = 48;
pub const EXIT_REASON_EPT_MISCONFIG: u32 = 49;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PinBasedControls: u32 {
        const EXTERNAL_INTERRUPT_EXITING = 1 << 0;
        const NMI_EXITING = 1 << 3;
        const VIRTUAL_NMIS = 1 << 5;
        const VMX_PREEMPTION_TIMER = 1 << 6;
        const PROCESS_POSTED_INTERRUPTS = 1 << 7;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PrimaryProcBasedControls: u32 {
        const INTERRUPT_WINDOW_EXITING = 1 << 2;
        const USE_TSC_OFFSETTING = 1 << 3;
        const HLT_EXITING = 1 << 7;
        const RDTSC_EXITING = 1 << 12;
        const CR3_LOAD_EXITING = 1 << 15;
        const CR3_STORE_EXITING = 1 << 16;
        const USE_MSR_BITMAPS = 1 << 28;
        const ACTIVATE_SECONDARY_CONTROLS = 1 << 31;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SecondaryProcBasedControls: u32 {
        const ENABLE_EPT = 1 << 1;
        const ENABLE_RDTSCP = 1 << 3;
        const ENABLE_VPID = 1 << 5;
        const UNRESTRICTED_GUEST = 1 << 7;
        const ENABLE_XSAVES = 1 << 20;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VmExitControls: u32 {
        const HOST_ADDRESS_SPACE_SIZE = 1 << 9; // Required for 64-bit host
        const LOAD_IA32_EFER = 1 << 21;
        const SAVE_IA32_EFER = 1 << 20;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VmEntryControls: u32 {
        const IA32E_MODE_GUEST = 1 << 9; // Required for 64-bit guest
        const LOAD_IA32_EFER = 1 << 15;
    }
}

pub unsafe fn adjust_vmx_control(msr: u32, mut val: u32) -> u32 {
    let cap = crate::cpu::rdmsr(msr);
    let allowed_0 = cap as u32;         // Bits that MUST be 1
    let allowed_1 = (cap >> 32) as u32; // Bits that CAN be 1
    val |= allowed_0;
    val &= allowed_1;
    val
}
