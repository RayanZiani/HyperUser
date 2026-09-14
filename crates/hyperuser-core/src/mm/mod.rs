use core::fmt;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub u64);

impl PhysAddr {
    pub fn as_u64(&self) -> u64 {
        self.0
    }
    
    pub fn from(val: u64) -> Self {
        Self(val)
    }
}

impl fmt::Display for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysAddr({:#x})", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub u64);

impl VirtAddr {
    pub fn as_u64(&self) -> u64 {
        self.0
    }
    
    pub fn from(val: u64) -> Self {
        Self(val)
    }
}

impl fmt::Display for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VirtAddr({:#x})", self.0)
    }
}

pub struct FrameAllocator {
    base: u64,         // Base physical address of managed region
    next: u64,         // Next free frame address
    limit: u64,        // End of managed region
}

impl FrameAllocator {
    pub fn new(base: u64, size: u64) -> Self {
        // Ensure base is 4KB aligned
        let aligned_base = (base + 0xFFF) & !0xFFF;
        Self {
            base: aligned_base,
            next: aligned_base,
            limit: base + size,
        }
    }

    pub fn alloc_frame(&mut self) -> Option<PhysAddr> {
        self.alloc_frames(1)
    }

    pub fn alloc_frames(&mut self, count: usize) -> Option<PhysAddr> {
        let size = (count as u64) * 0x1000;
        let start = self.next;
        let end = start.checked_add(size)?;

        if end <= self.limit {
            self.next = end;
            Some(PhysAddr(start))
        } else {
            None
        }
    }
}

pub static FRAME_ALLOCATOR: Mutex<Option<FrameAllocator>> = Mutex::new(None);

pub fn init_allocator(base: u64, size: u64) {
    *FRAME_ALLOCATOR.lock() = Some(FrameAllocator::new(base, size));
}

pub fn alloc_frame() -> Option<PhysAddr> {
    if let Some(allocator) = FRAME_ALLOCATOR.lock().as_mut() {
        allocator.alloc_frame()
    } else {
        None
    }
}
