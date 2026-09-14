pub mod intel;
pub mod amd;

use hyperuser_proto::registers::GuestRegisters;

/// Error type for hypervisor operations
#[derive(Debug, Clone, Copy)]
pub enum HvError {
    /// CPU does not support required virtualization extensions
    NotSupported,
    /// VMX/SVM instruction failed
    VmInstructionFailed,
    /// Memory allocation failure
    OutOfMemory,
    /// Invalid VMCS/VMCB state
    InvalidState,
    /// Feature control MSR is locked
    FeatureControlLocked,
    /// Generic error with message
    Generic(&'static str),
}

/// Action to take after handling a VM-Exit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitAction {
    /// Continue guest execution (vmresume)
    Continue,
    /// Continue but advance guest RIP past the faulting instruction
    IncrementRip(u64),
    /// Shutdown the hypervisor and restore bare-metal execution  
    Shutdown,
}

/// Trait abstracting vendor-specific hypervisor operations
pub trait Hypervisor {
    /// Check if this vendor's virtualization is supported on the current CPU
    fn is_supported() -> bool;
    /// Enable virtualization extensions (CR4.VMXE / EFER.SVME)
    fn enable() -> Result<(), HvError>;
    /// Allocate and initialize virtualization structures (VMCS/VMCB, VMXON region, etc.)
    fn initialize() -> Result<(), HvError>;
    /// Launch the virtual machine
    fn launch(guest_regs: &mut GuestRegisters) -> Result<(), HvError>;
}
