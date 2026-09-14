#![no_std]

pub mod registers;

pub const HYPERUSER_MAGIC: u64 = 0x4879_7065_7255_7372;
pub const HYPERCALL_LEAF: u32 = 0x4879_7055;
pub const HYPERUSER_SIGNATURE: [u8; 4] = *b"HypU";

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    Ping = 0,
    ReadPhys = 1,
    WritePhys = 2,
    HookPage = 3,
    UnhookPage = 4,
    GetStatus = 5,
    Shutdown = 6,
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Success = 0,
    InvalidToken = 1,
    InvalidCommand = 2,
    AccessDenied = 3,
    InvalidAddress = 4,
    Fault = 0xFFFF,
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct HypercallPacket {
    pub magic_token: u64,
    pub command: CommandId,
    pub _pad0: u32,
    pub status: u64,
    pub target_cr3: u64,
    pub source_va: u64,
    pub target_pa: u64,
    pub size: u64,
    pub payload: [u8; 256],
}

impl HypercallPacket {
    pub fn new(command: CommandId) -> Self {
        Self {
            magic_token: HYPERUSER_MAGIC,
            command,
            _pad0: 0,
            status: StatusCode::Success as u64,
            target_cr3: 0,
            source_va: 0,
            target_pa: 0,
            size: 0,
            payload: [0; 256],
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic_token == HYPERUSER_MAGIC
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    Intel,
    Amd,
    Unknown,
}
