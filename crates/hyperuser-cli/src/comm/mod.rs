//! Direct hypervisor communication via overloaded CPUID instructions.
//!
//! The client issues a CPUID instruction with specially crafted register values:
//! - EAX: `HYPERCALL_LEAF` (magic CPUID leaf identifier)
//! - EBX: Magic token (lower 32 bits of `HYPERUSER_MAGIC`)
//! - RCX: Virtual address of the `HypercallPacket`
//! - EDX: Command ID hash
//!
//! The hypervisor intercepts the resulting VM-Exit (`EXIT_REASON_CPUID`),
//! validates the magic token, reads the packet from guest memory, processes
//! the command, and writes results back — all transparently.

use hyperuser_proto::{HypercallPacket, HYPERCALL_LEAF, HYPERUSER_MAGIC};

/// Errors that can occur during hypercall communication.
#[derive(Debug)]
pub enum CliError {
    /// Failed to allocate aligned memory for the packet
    AllocationFailed,
    /// The CPUID hypercall dispatch failed
    HypercallFailed,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::AllocationFailed => write!(f, "Failed to allocate aligned memory"),
            CliError::HypercallFailed => write!(f, "Hypercall execution failed"),
        }
    }
}

impl std::error::Error for CliError {}

/// Execute a hypercall by issuing an overloaded CPUID instruction.
///
/// The hypervisor intercepts CPUID exits and checks for our magic values
/// in the guest registers. If the magic token matches, it reads the
/// [`HypercallPacket`] from the virtual address carried in RCX, processes
/// the command, and writes the response back into the packet.
///
/// # Register Layout
///
/// | Register | Value |
/// |----------|-------|
/// | EAX | `HYPERCALL_LEAF` (magic leaf) |
/// | EBX | Lower 32 bits of `HYPERUSER_MAGIC` |
/// | RCX | Virtual address of `HypercallPacket` |
/// | EDX | `CommandId` as u32 |
pub fn hypercall(packet: &mut HypercallPacket) -> Result<(), CliError> {
    // Stamp the magic token so the hypervisor recognizes this CPUID as a hypercall
    packet.magic_token = HYPERUSER_MAGIC;

    let packet_va = packet as *mut HypercallPacket as u64;
    let magic_lo = HYPERUSER_MAGIC as u32;
    let cmd_hash = packet.command as u32;

    unsafe {
        // We use inline asm for full register control. The standard __cpuid
        // intrinsic doesn't let us set RCX to an arbitrary 64-bit pointer
        // (it treats ECX as a subleaf index).
        //
        // The hypervisor reads the full 64-bit RCX from the guest register
        // state saved in the VMCS/VMCB on VM-Exit.
        core::arch::asm!(
            // Save EBX — it's callee-saved on both Windows and SysV x64 ABIs
            "mov {tmp:r}, rbx",
            // Load our magic token into EBX
            "mov ebx, {magic:e}",
            // Execute CPUID — triggers VM-Exit in the hypervisor
            "cpuid",
            // Restore EBX
            "mov rbx, {tmp:r}",
            // Inputs
            magic = in(reg) magic_lo,
            tmp = out(reg) _,
            // EAX = hypercall leaf
            inout("eax") HYPERCALL_LEAF => _,
            // RCX = full 64-bit virtual address of the packet
            inout("rcx") packet_va => _,
            // EDX = command ID
            inout("edx") cmd_hash => _,
            options(nostack, preserves_flags)
        );
    }

    Ok(())
}
