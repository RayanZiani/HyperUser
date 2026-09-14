use hyperuser_proto::CpuVendor;

/// CPUID instruction result
#[derive(Debug, Clone, Copy)]
pub struct CpuIdResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

/// Executes the `cpuid` instruction safely by preserving `ebx`.
#[inline(always)]
pub unsafe fn cpuid(leaf: u32, subleaf: u32) -> CpuIdResult {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    core::arch::asm!(
        "mov {tmp:e}, ebx",
        "cpuid",
        "xchg {tmp:e}, ebx",
        tmp = out(reg) ebx,
        inlateout("eax") leaf => eax,
        inlateout("ecx") subleaf => ecx,
        lateout("edx") edx,
        options(nomem, preserves_flags)
    );
    CpuIdResult { eax, ebx, ecx, edx }
}

/// Reads a 64-bit value from a Model-Specific Register (MSR).
#[inline(always)]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Writes a 64-bit value to a Model-Specific Register (MSR).
#[inline(always)]
pub unsafe fn wrmsr(msr: u32, val: u64) {
    let low = val as u32;
    let high = (val >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, preserves_flags)
    );
}

#[inline(always)]
pub unsafe fn read_cr0() -> u64 {
    let cr0: u64;
    core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nomem, preserves_flags));
    cr0
}

#[inline(always)]
pub unsafe fn write_cr0(val: u64) {
    core::arch::asm!("mov cr0, {}", in(reg) val, options(nomem, preserves_flags));
}

#[inline(always)]
pub unsafe fn read_cr3() -> u64 {
    let cr3: u64;
    core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, preserves_flags));
    cr3
}

#[inline(always)]
pub unsafe fn write_cr3(val: u64) {
    core::arch::asm!("mov cr3, {}", in(reg) val, options(nomem, preserves_flags));
}

#[inline(always)]
pub unsafe fn read_cr4() -> u64 {
    let cr4: u64;
    core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, preserves_flags));
    cr4
}

#[inline(always)]
pub unsafe fn write_cr4(val: u64) {
    core::arch::asm!("mov cr4, {}", in(reg) val, options(nomem, preserves_flags));
}

#[inline(always)]
pub unsafe fn read_rflags() -> u64 {
    let rflags: u64;
    core::arch::asm!(
        "pushfq",
        "pop {}",
        out(reg) rflags,
        options(nomem, preserves_flags)
    );
    rflags
}

/// Detects the CPU vendor based on `cpuid` leaf 0.
#[inline(always)]
pub fn detect_vendor() -> CpuVendor {
    unsafe {
        let res = cpuid(0, 0);
        let mut vendor = [0u8; 12];
        vendor[0..4].copy_from_slice(&res.ebx.to_le_bytes());
        vendor[4..8].copy_from_slice(&res.edx.to_le_bytes());
        vendor[8..12].copy_from_slice(&res.ecx.to_le_bytes());
        
        match &vendor {
            b"GenuineIntel" => CpuVendor::Intel,
            b"AuthenticAMD" => CpuVendor::Amd,
            _ => CpuVendor::Unknown,
        }
    }
}

/// Checks if Intel VT-x (VMX) is supported by the CPU.
#[inline(always)]
pub fn is_vmx_supported() -> bool {
    unsafe { (cpuid(1, 0).ecx & (1 << 5)) != 0 }
}

/// Checks if AMD-V (SVM) is supported by the CPU.
#[inline(always)]
pub fn is_svm_supported() -> bool {
    unsafe { (cpuid(0x8000_0001, 0).ecx & (1 << 2)) != 0 }
}

pub const IA32_FEATURE_CONTROL: u32 = 0x3A;
pub const IA32_VMX_BASIC: u32 = 0x480;
pub const IA32_VMX_PINBASED_CTLS: u32 = 0x481;
pub const IA32_VMX_PROCBASED_CTLS: u32 = 0x482;
pub const IA32_VMX_EXIT_CTLS: u32 = 0x483;
pub const IA32_VMX_ENTRY_CTLS: u32 = 0x484;
pub const IA32_VMX_PROCBASED_CTLS2: u32 = 0x48B;
pub const IA32_VMX_TRUE_PINBASED_CTLS: u32 = 0x48D;
pub const IA32_VMX_TRUE_PROCBASED_CTLS: u32 = 0x48E;
pub const IA32_VMX_TRUE_EXIT_CTLS: u32 = 0x48F;
pub const IA32_VMX_TRUE_ENTRY_CTLS: u32 = 0x490;
pub const IA32_EFER: u32 = 0xC0000080;
pub const IA32_VMX_CR0_FIXED0: u32 = 0x486;
pub const IA32_VMX_CR0_FIXED1: u32 = 0x487;
pub const IA32_VMX_CR4_FIXED0: u32 = 0x488;
pub const IA32_VMX_CR4_FIXED1: u32 = 0x489;
pub const MSR_VM_CR: u32 = 0xC001_0114;
pub const MSR_VM_HSAVE_PA: u32 = 0xC001_0117;

/// Enables Intel VT-x operation.
#[inline(always)]
pub unsafe fn enable_vmx() -> Result<(), &'static str> {
    let cr4 = read_cr4();
    write_cr4(cr4 | (1 << 13)); // Set CR4.VMXE
    
    let feature_control = rdmsr(IA32_FEATURE_CONTROL);
    if (feature_control & 1) == 0 {
        // Not locked, lock it with VMXON outside SMX
        wrmsr(IA32_FEATURE_CONTROL, feature_control | (1 << 2) | 1);
    } else {
        // Locked
        if (feature_control & (1 << 2)) == 0 {
            return Err("VMX locked off by BIOS");
        }
    }
    
    Ok(())
}
