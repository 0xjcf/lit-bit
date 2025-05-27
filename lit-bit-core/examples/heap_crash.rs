#![cfg_attr(any(target_arch = "riscv32", target_arch = "arm"), no_std)]
#![cfg_attr(any(target_arch = "riscv32", target_arch = "arm"), no_main)]

// Dummy allocator: satisfies linker for potential `alloc` references,
// but will crash if actual heap allocation occurs.
#[cfg(any(target_arch = "riscv32", target_arch = "arm"))]
#[global_allocator]
static DUMMY: DummyAlloc = DummyAlloc;

#[cfg(any(target_arch = "riscv32", target_arch = "arm"))]
struct DummyAlloc;

#[cfg(any(target_arch = "riscv32", target_arch = "arm"))]
unsafe impl core::alloc::GlobalAlloc for DummyAlloc {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        // Panic immediately to prevent undefined behavior from null pointer dereference
        panic!("DummyAlloc: heap allocation attempted in no_std context")
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[cfg(target_arch = "riscv32")]
extern crate alloc;
#[cfg(target_arch = "riscv32")]
use alloc::boxed::Box;
#[cfg(target_arch = "riscv32")]
use panic_halt as _;
#[cfg(target_arch = "riscv32")]
use riscv_rt::entry;

#[cfg(target_arch = "riscv32")]
#[entry]
fn main() -> ! {
    // Attempt to allocate on the heap — should crash due to dummy allocator
    let _leaked = Box::leak(Box::new(1234u32));
    loop {}
}

#[cfg(target_arch = "arm")]
extern crate alloc;
#[cfg(target_arch = "arm")]
use alloc::boxed::Box;
#[cfg(target_arch = "arm")]
use cortex_m_rt::entry;
#[cfg(target_arch = "arm")]
use panic_halt as _;

#[cfg(target_arch = "arm")]
#[entry]
fn main() -> ! {
    // Attempt to allocate on the heap — should crash due to dummy allocator
    let _leaked = Box::leak(Box::new(5678u32));
    loop {}
}

#[cfg(not(any(target_arch = "riscv32", target_arch = "arm")))]
fn main() {
    println!(
        "This example is only meant to be built and run for the `riscv32` or `arm` (Cortex-M) targets."
    );
}
