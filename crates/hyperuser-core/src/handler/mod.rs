//! VM-Exit handler dispatch.
//!
//! This module contains the main VM-Exit dispatcher and individual handlers
//! for each intercepted exit reason. The dispatcher is called from the
//! VM-Exit assembly stub after guest registers have been saved.

use hyperuser_proto::registers::GuestRegisters;
use hyperuser_proto::{CommandId, HypercallPacket, StatusCode, HYPERCALL_LEAF, HYPERUSER_MAGIC};
use crate::arch::ExitAction;
use crate::cpu;
use crate::vmx::vmcs;

/// Main VM-Exit dispatcher. Called from the VM-Exit assembly stub
/// after guest registers have been saved to the `GuestRegisters` structure.
///
/// Reads the exit reason from the VMCS and dispatches to the appropriate
/// handler function. Returns an [`ExitAction`] indicating how to resume.
pub fn handle_vm_exit(guest_regs: &mut GuestRegisters) -> ExitAction {
    let exit_reason = unsafe {
        crate::vmx::vmread(vmcs::VM_EXIT_REASON).unwrap_or(0xFFFF)
    } as u32;
    let basic_reason = exit_reason & 0xFFFF; // Lower 16 bits

    match basic_reason {
        vmcs::EXIT_REASON_CPUID => handle_cpuid(guest_regs),
        vmcs::EXIT_REASON_MSR_READ => handle_msr_read(guest_regs),
        vmcs::EXIT_REASON_MSR_WRITE => handle_msr_write(guest_regs),
        vmcs::EXIT_REASON_CR_ACCESS => handle_cr_access(guest_regs),
        vmcs::EXIT_REASON_EPT_VIOLATION => handle_ept_violation(guest_regs),
        vmcs::EXIT_REASON_VMCALL => handle_vmcall(guest_regs),
        _ => {
            log::warn!("[!] Unhandled VM-Exit reason: {:#x}", basic_reason);
            ExitAction::Continue
        }
    }
}

/// Handle CPUID VM-Exit.
///
/// Checks if the guest issued a CPUID with our magic leaf + token.
/// If recognized as a hypercall, reads the `HypercallPacket` from guest
/// memory, dispatches the command, and writes the response.
/// Otherwise, performs a real CPUID passthrough.
fn handle_cpuid(guest_regs: &mut GuestRegisters) -> ExitAction {
    let leaf = guest_regs.rax as u32;
    let magic = guest_regs.rbx as u32;

    if leaf == HYPERCALL_LEAF && magic == (HYPERUSER_MAGIC as u32) {
        // This is a hypercall from the CLI.
        // RCX contains the virtual address of the HypercallPacket.
        let packet_va = guest_regs.rcx;
        let guest_cr3 = unsafe {
            crate::vmx::vmread(vmcs::GUEST_CR3).unwrap_or(0)
        };

        // Translate the guest VA to a host physical address using the
        // guest page table walker, then access the packet.
        if let Some(packet_pa) = crate::paging::translate_guest_va(guest_cr3, packet_va) {
            let packet = unsafe { &mut *(packet_pa as *mut HypercallPacket) };

            if packet.is_valid() {
                dispatch_hypercall(packet, guest_regs);
            } else {
                packet.status = StatusCode::InvalidToken as u64;
            }
        }
    } else {
        // Standard CPUID passthrough — execute real CPUID and return results
        let subleaf = guest_regs.rcx as u32;
        let result = unsafe { cpu::cpuid(leaf, subleaf) };
        guest_regs.rax = result.eax as u64;
        guest_regs.rbx = result.ebx as u64;
        guest_regs.rcx = result.ecx as u64;
        guest_regs.rdx = result.edx as u64;
    }

    let instr_len = get_exit_instruction_length();
    ExitAction::IncrementRip(instr_len)
}

/// Dispatch a validated hypercall command.
fn dispatch_hypercall(packet: &mut HypercallPacket, _guest_regs: &mut GuestRegisters) {
    match packet.command {
        CommandId::Ping => {
            // Simple liveness check — just set status to success
            packet.status = StatusCode::Success as u64;
        }
        CommandId::ReadPhys => {
            // TODO: Phase 2 — read `packet.size` bytes from `packet.target_pa`
            // into `packet.payload`.
            packet.status = StatusCode::Success as u64;
        }
        CommandId::WritePhys => {
            // TODO: Phase 2 — write `packet.size` bytes from `packet.payload`
            // to `packet.target_pa`.
            packet.status = StatusCode::Success as u64;
        }
        CommandId::HookPage => {
            // TODO: Phase 2 — split EPT 2MB page and set execute-only
            packet.status = StatusCode::Success as u64;
        }
        CommandId::UnhookPage => {
            // TODO: Phase 2 — restore original EPT mapping
            packet.status = StatusCode::Success as u64;
        }
        CommandId::GetStatus => {
            // Return basic status info in payload
            packet.status = StatusCode::Success as u64;
        }
        CommandId::Shutdown => {
            // TODO: Phase 2 — disable VMX and return to bare-metal
            packet.status = StatusCode::Success as u64;
        }
    }
}

/// Handle MSR read VM-Exit (RDMSR).
///
/// Spoofs `IA32_FEATURE_CONTROL` to hide VMX from the guest OS.
/// All other MSR reads are passed through to real hardware.
fn handle_msr_read(guest_regs: &mut GuestRegisters) -> ExitAction {
    let msr_index = guest_regs.rcx as u32;

    let value = match msr_index {
        cpu::IA32_FEATURE_CONTROL => {
            // Spoof: report VMX as locked/disabled by BIOS
            // Bit 0 = Lock, Bit 2 = VMXON outside SMX = 0 (disabled)
            0x1_u64 // Locked, but VMX disabled
        }
        _ => {
            // Passthrough: execute real RDMSR
            unsafe { cpu::rdmsr(msr_index) }
        }
    };

    // RDMSR returns result in EDX:EAX
    guest_regs.rax = value & 0xFFFF_FFFF;
    guest_regs.rdx = value >> 32;

    let instr_len = get_exit_instruction_length();
    ExitAction::IncrementRip(instr_len)
}

/// Handle MSR write VM-Exit (WRMSR).
///
/// Currently passes all writes through to real hardware.
fn handle_msr_write(guest_regs: &mut GuestRegisters) -> ExitAction {
    let msr_index = guest_regs.rcx as u32;
    let value = (guest_regs.rdx << 32) | (guest_regs.rax & 0xFFFF_FFFF);

    unsafe {
        cpu::wrmsr(msr_index, value);
    }

    let instr_len = get_exit_instruction_length();
    ExitAction::IncrementRip(instr_len)
}

/// Handle control register access VM-Exit.
///
/// Placeholder for CR4 read shadow logic — the VMCS CR4 guest/host mask
/// and read shadow handle most cases automatically.
fn handle_cr_access(_guest_regs: &mut GuestRegisters) -> ExitAction {
    // The VMCS CR4 Guest/Host Mask + Read Shadow handles CR4.VMXE
    // transparently in hardware. This handler is for edge cases.
    let _qualification = unsafe {
        crate::vmx::vmread(vmcs::VM_EXIT_QUALIFICATION).unwrap_or(0)
    };
    // TODO: Phase 2 — parse qualification to determine CR number and access type
    ExitAction::Continue
}

/// Handle EPT violation VM-Exit.
///
/// Placeholder for the EPT fault channel used for page-level hooks.
fn handle_ept_violation(_guest_regs: &mut GuestRegisters) -> ExitAction {
    let _qualification = unsafe {
        crate::vmx::vmread(vmcs::VM_EXIT_QUALIFICATION).unwrap_or(0)
    };
    let _guest_pa = unsafe {
        crate::vmx::vmread(vmcs::GUEST_PHYSICAL_ADDRESS).unwrap_or(0)
    };
    // TODO: Phase 2 — check if this is a hooked page, swap EPT view
    log::warn!("[!] EPT violation at GPA {:#x}", _guest_pa);
    ExitAction::Continue
}

/// Handle VMCALL VM-Exit.
///
/// Alternative hypercall entry point (used by kernel-mode clients).
fn handle_vmcall(_guest_regs: &mut GuestRegisters) -> ExitAction {
    // TODO: Phase 2 — parse VMCALL parameters for kernel-mode hypercalls
    let instr_len = get_exit_instruction_length();
    ExitAction::IncrementRip(instr_len)
}

/// Read the instruction length of the faulting instruction from the VMCS.
pub fn get_exit_instruction_length() -> u64 {
    unsafe { crate::vmx::vmread(vmcs::VM_EXIT_INSTR_LEN).unwrap_or(0) }
}
