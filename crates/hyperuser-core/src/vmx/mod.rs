use core::arch::asm;

pub mod ept;
pub mod vmcs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmxError {
    VmFailInvalid,
    VmFailValid(u32),
}

#[inline(always)]
pub unsafe fn vmxon(vmxon_region_pa: u64) -> Result<(), VmxError> {
    let cf: u8;
    let zf: u8;
    asm!(
        "vmxon [{pa}]",
        "setc {cf}",
        "setz {zf}",
        pa = in(reg) &vmxon_region_pa,
        cf = out(reg_byte) cf,
        zf = out(reg_byte) zf,
        options(nostack)
    );
    if cf != 0 { return Err(VmxError::VmFailInvalid); }
    if zf != 0 { return Err(VmxError::VmFailValid(0)); }
    Ok(())
}

#[inline(always)]
pub unsafe fn vmxoff() -> Result<(), VmxError> {
    let cf: u8;
    let zf: u8;
    asm!(
        "vmxoff",
        "setc {cf}",
        "setz {zf}",
        cf = out(reg_byte) cf,
        zf = out(reg_byte) zf,
        options(nostack)
    );
    if cf != 0 { return Err(VmxError::VmFailInvalid); }
    if zf != 0 { return Err(VmxError::VmFailValid(0)); }
    Ok(())
}

#[inline(always)]
pub unsafe fn vmclear(vmcs_pa: u64) -> Result<(), VmxError> {
    let cf: u8;
    let zf: u8;
    asm!(
        "vmclear [{pa}]",
        "setc {cf}",
        "setz {zf}",
        pa = in(reg) &vmcs_pa,
        cf = out(reg_byte) cf,
        zf = out(reg_byte) zf,
        options(nostack)
    );
    if cf != 0 { return Err(VmxError::VmFailInvalid); }
    if zf != 0 { return Err(VmxError::VmFailValid(0)); }
    Ok(())
}

#[inline(always)]
pub unsafe fn vmptrld(vmcs_pa: u64) -> Result<(), VmxError> {
    let cf: u8;
    let zf: u8;
    asm!(
        "vmptrld [{pa}]",
        "setc {cf}",
        "setz {zf}",
        pa = in(reg) &vmcs_pa,
        cf = out(reg_byte) cf,
        zf = out(reg_byte) zf,
        options(nostack)
    );
    if cf != 0 { return Err(VmxError::VmFailInvalid); }
    if zf != 0 { return Err(VmxError::VmFailValid(0)); }
    Ok(())
}

#[inline(always)]
pub unsafe fn vmread(field: u32) -> Result<u64, VmxError> {
    let cf: u8;
    let zf: u8;
    let mut val: u64;
    asm!(
        "vmread {dest}, {src}",
        "setc {cf}",
        "setz {zf}",
        src = in(reg) field as u64,
        dest = out(reg) val,
        cf = out(reg_byte) cf,
        zf = out(reg_byte) zf,
        options(nostack)
    );
    if cf != 0 { return Err(VmxError::VmFailInvalid); }
    if zf != 0 { return Err(VmxError::VmFailValid(0)); }
    Ok(val)
}

#[inline(always)]
pub unsafe fn vmwrite(field: u32, val: u64) -> Result<(), VmxError> {
    let cf: u8;
    let zf: u8;
    asm!(
        "vmwrite {dest}, {src}",
        "setc {cf}",
        "setz {zf}",
        src = in(reg) val,
        dest = in(reg) field as u64,
        cf = out(reg_byte) cf,
        zf = out(reg_byte) zf,
        options(nostack)
    );
    if cf != 0 { return Err(VmxError::VmFailInvalid); }
    if zf != 0 { 
        let err = vmread(0x4400).unwrap_or(0);
        return Err(VmxError::VmFailValid(err as u32)); 
    }
    Ok(())
}

#[inline(always)]
pub unsafe fn vmlaunch() -> Result<(), VmxError> {
    let cf: u8;
    let zf: u8;
    asm!(
        "vmlaunch",
        "setc {cf}",
        "setz {zf}",
        cf = out(reg_byte) cf,
        zf = out(reg_byte) zf,
        options(nostack)
    );
    if cf != 0 { return Err(VmxError::VmFailInvalid); }
    if zf != 0 { 
        let err = vmread(0x4400).unwrap_or(0);
        return Err(VmxError::VmFailValid(err as u32)); 
    }
    Ok(())
}

#[inline(always)]
pub unsafe fn vmresume() -> Result<(), VmxError> {
    let cf: u8;
    let zf: u8;
    asm!(
        "vmresume",
        "setc {cf}",
        "setz {zf}",
        cf = out(reg_byte) cf,
        zf = out(reg_byte) zf,
        options(nostack)
    );
    if cf != 0 { return Err(VmxError::VmFailInvalid); }
    if zf != 0 { 
        let err = vmread(0x4400).unwrap_or(0);
        return Err(VmxError::VmFailValid(err as u32)); 
    }
    Ok(())
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InvEptDescriptor {
    pub eptp: u64,
    pub reserved: u64,
}

#[inline(always)]
pub unsafe fn invept(inv_type: u64, descriptor: &InvEptDescriptor) -> Result<(), VmxError> {
    let cf: u8;
    let zf: u8;
    asm!(
        "invept {reg}, [{desc}]",
        "setc {cf}",
        "setz {zf}",
        reg = in(reg) inv_type,
        desc = in(reg) descriptor,
        cf = out(reg_byte) cf,
        zf = out(reg_byte) zf,
        options(nostack)
    );
    if cf != 0 { return Err(VmxError::VmFailInvalid); }
    if zf != 0 { return Err(VmxError::VmFailValid(0)); }
    Ok(())
}
