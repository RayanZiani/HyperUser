# HyperUser

Customer Hypervisor written in **Rust** with **driverless access** from Windows **user-mode**.

# HyperUser — Minimalist Bare-Metal Type-1 Hypervisor & Ring-3 Telemetry Framework

A research-oriented, dual-vendor (Intel VT-x / AMD-V) Type-1 hypervisor and zero-driver user-mode telemetry framework written in `no_std` Rust.

---

## 1. Architectural Overview

hyperuser consists of two modular crates designed to operate across the OS/Hardware boundary:

1. **`hyperuser-core` (UEFI / Type-1 Hypervisor):** Boots prior to the operating system, claims hardware virtualization extensions (VMX/SVM), manages Second-Level Address Translation (SLAT / EPT / NPT), sanitizes firmware memory footprints, and handles guest VM-Exits transparently.
2. **`hyperuser-cli` (User-Mode Interface):** A zero-driver client that communicates with the hypervisor directly from Ring 3 using overloaded CPUID handshakes, zero-copy shared physical memory mappings, or EPT fault channels without relying on custom Windows kernel drivers (`.sys`).

```text
+-------------------------------------------------------------+
|               User Space (Ring 3)                           |
|  [ hyperuser-cli ] (CLI / Telemetry / Memory Scanner)       |
+-------------------------------------------------------------+
│ (Overloaded CPUID / Shared Buffer / EPT Fault)
▼
================== OS Kernel Space (Ring 0) ==================
[ Windows / Linux Kernel ] (Completely bypassed / Passthrough)

     │ (Hardware VM-Exit / Host Trap)
     ▼

+-------------------------------------------------------------+
|               Host / Hypervisor Mode (Ring -1)              |
|  [ hyperuser-core ]                                         |
|    ├── VMX / SVM Engine (Trait-based abstraction)           |
|    ├── SLAT / EPT Manager (2MB / 4KB Split Engine)          |
|    ├── CR3 Guest Page Table Walker                          |
|    ├── Shadow MSR & CR4 Read Registers                      |
|    └── Dynamic TSC Offsetting Engine                        |
+-------------------------------------------------------------+
```

---

## 2. Repository Layout

```text
hyperuser/
├── Cargo.toml                  # Cargo Workspace configuration
├── Makefile.toml               # Build & image generation workflows (cargo-make)
├── crates/
│   ├── hyperuser-core/         # Type-1 Hypervisor EFI Binary (no_std)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs         # EFI entry point & ExitBootServices hook
│   │       ├── arch/
│   │       │   ├── mod.rs      # Hypervisor Trait definitions
│   │       │   ├── intel/      # Intel VT-x (VMCS, EPT, VMXON)
│   │       │   └── amd/        # AMD-V (VMCB, NPT, VMRUN)
│   │       ├── paging/         # 4-Level/5-Level CR3 table walker
│   │       ├── mm/             # Physical memory allocator & EPT manager
│   │       └── handler/        # VM-Exit dispatchers & stealth controls
│   ├── hyperuser-cli/          # Ring 3 User-Mode CLI Application
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs         # Command processor & REPL
│   │       ├── comm/           # Direct assembly CPUID dispatcher
│   │       └── memory/         # Virtual-to-physical translation helpers
│   └── hyperuser-proto/        # Shared Structures & Command Packets (no_std)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs          # Command codes, packet layouts, rolling tokens
│           └── registers.rs    # CPU register snapshot structs
```

---

## 3. Communication Protocol Specification

Communication operates without Windows kernel drivers by executing CPU instruction traps directly from Ring 3:

**Command Packet Structure (`hyperuser-proto`)**

```rust
#[repr(C, align(16))]
pub struct HypercallPacket {
    pub magic_token: u64,      // Rolling cryptotoken / session ID
    pub command: CommandId,    // Enum: ReadPhys, WritePhys, HookPage, GetStatus
    pub status: u64,           // Status return code (0 = Success)
    pub target_cr3: u64,       // Optional: CR3 of target process
    pub source_va: u64,        // Virtual address in user space
    pub target_pa: u64,        // Physical address to access
    pub size: usize,           // Byte count
    pub payload: [u8; 256],    // In-line small buffer / response area
}
```

**Protocol Sequence**

1. **User Mode Allocation**: The client allocates a 16-byte aligned packet via `VirtualAlloc`.
2. **Registration / Dispatch**: The user mode app populates register parameters:
    * **EAX**: Base CPUID leaf (0x1 or 0x7 overloaded leaf).
    * **EBX**: Magic token / Sequence counter.
    * **RCX**: Virtual address of `HypercallPacket`.
    * **EDX**: Command action hash.
3. **VM-Exit Interception**:
    * Host traps on `EXIT_REASON_CPUID`.
    * Host checks validation token in registers. If invalid, standard CPUID output is returned to guest registers without modification.
    * If valid, host accesses guest CR3, translates RCX (Virtual Address) to Host Physical Address (HPA), performs the requested operation, writes the result to `packet.payload`, advances Guest RIP, and resumes guest execution.

---

## 4. Anti-Introspection & Transparency Matrix

| Vector | Mechanism | Countermeasure in `hyperuser-core` |
| :--- | :--- | :--- |
| **Timing Invalidation** | RDTSC / RDTSCP delta measurements across exits | Dynamically subtract VM-Exit overhead cycles from the VMCS `TSC_OFFSET` field or trap and scale RDTSC ticks. |
| **Control Register Auditing** | `mov rax, cr4` checks for CR4.VMXE (Bit 13) | Configure CR4 Guest/Host Mask and set CR4 Read Shadow to report VMXE = 0. |
| **MSR Verification** | Querying `IA32_FEATURE_CONTROL` (0x3A) | Intercept MSR reads in the MSR bitmap; return BIOS-locked / disabled virtualization flags. |
| **Memory Map Traversal** | OS scanning EFI Runtime / Reserved descriptors | Hook `EFI_BOOT_SERVICES.ExitBootServices`; rewrite descriptor tables to mark hypervisor pages as conventional RAM. |
| **Second-Level Access** | Ring 0 memory scans targeting hypervisor pages | Mark hypervisor physical pages as R=0 / W=0 / X=0 inside the EPT/NPT to trigger an EPT violation if probed. |

---

## 5. Prerequisites & Toolchain Setup

**Target Dependencies**

*   **Rust Toolchain**: `nightly` (for `core::arch::asm`, `no_std` allocator APIs, and custom alloc error handlers).
*   **Target Architectures**:
    *   `x86_64-unknown-uefi` (Hypervisor bootloader)
    *   `x86_64-pc-windows-msvc` or `x86_64-unknown-linux-gnu` (User-mode client)
*   **Tools**: `cargo-make`, `llvm-tools-preview`, `ovmf` (for QEMU testing), and `qemu-system-x86_64`.

```bash
rustup install nightly
rustup default nightly
rustup component add rust-src llvm-tools-preview
rustup target add x86_64-unknown-uefi x86_64-pc-windows-msvc
cargo install cargo-make
```

---

## 6. Build and Deployment Instructions

### 6.1 Compiling the Workspace

```bash
# Build the UEFI Hypervisor core in release mode
cargo build --release -p hyperuser-core --target x86_64-unknown-uefi

# Build the Ring-3 CLI utility
cargo build --release -p hyperuser-cli --target x86_64-pc-windows-msvc
```

### 6.2 Testing in QEMU with OVMF

Create a minimal FAT filesystem image with the hypervisor placed at `EFI\BOOT\BOOTX64.EFI`:

```bash
mkdir -p build/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/hyperuser_core.efi build/esp/EFI/BOOT/BOOTX64.EFI

# Launch QEMU with nested virtualization and OVMF firmware
qemu-system-x86_64 \
    -enable-kvm \
    -cpu host,vmx=on \
    -m 4G \
    -bios /usr/share/OVMF/OVMF_CODE.fd \
    -drive format=raw,file=fat:rw:build/esp \
    -nographic
```

### 6.3 Deploying to Physical Hardware

1. Format a USB drive as FAT32.
2. Copy `hyperuser_core.efi` to `X:\EFI\BOOT\BOOTX64.EFI`.
3. In your motherboard firmware settings:
    * Enable Intel Virtualization Technology (VT-x) or AMD SVM.
    * Disable Secure Boot (or sign the EFI binary with a custom machine owner key / MOK).
    * Disable VBS / Hyper-V in Windows (`bcdedit /set hypervisorlaunchtype off`).
4. Boot from the USB drive. The hypervisor initializes, hooks `ExitBootServices`, enables virtualization, and chains load execution into the primary OS bootloader (`bootmgfw.efi` or `grubx64.efi`).
5. Once inside the OS, execute `hyperuser-cli.exe` to manage hypervisor state directly from Ring 3.

---

## Next Steps

* Generate complete Cargo.toml workspace configuration
* Show hyperuser-proto crate implementation
* Implement the UEFI entry point and ExitBootServices hook