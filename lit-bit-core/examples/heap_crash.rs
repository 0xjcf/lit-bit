#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

// Dummy allocator: satisfies linker for potential `alloc` references,
// but will crash if actual heap allocation occurs.
#[cfg(not(feature = "std"))]
#[global_allocator]
static DUMMY: DummyAlloc = DummyAlloc;

#[cfg(not(feature = "std"))]
struct DummyAlloc;

#[cfg(not(feature = "std"))]
unsafe impl core::alloc::GlobalAlloc for DummyAlloc {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        // Panic immediately to prevent undefined behavior from null pointer dereference
        panic!("DummyAlloc: heap allocation attempted in no_std context")
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[cfg(all(not(feature = "std"), target_arch = "riscv32"))]
extern crate alloc;
#[cfg(all(not(feature = "std"), target_arch = "riscv32"))]
use alloc::boxed::Box;
#[cfg(all(not(feature = "std"), target_arch = "riscv32"))]
use panic_halt as _;
#[cfg(all(not(feature = "std"), target_arch = "riscv32"))]
use riscv_rt::entry;

#[cfg(all(not(feature = "std"), target_arch = "riscv32"))]
#[entry]
fn main() -> ! {
    // Attempt to allocate on the heap — should crash due to dummy allocator
    let _leaked = Box::leak(Box::new(1234u32));
    #[allow(clippy::empty_loop)]
    loop {}
}

#[cfg(all(not(feature = "std"), target_arch = "arm"))]
extern crate alloc;
#[cfg(all(not(feature = "std"), target_arch = "arm"))]
use alloc::boxed::Box;
#[cfg(all(not(feature = "std"), target_arch = "arm"))]
use cortex_m_rt::entry;
#[cfg(all(not(feature = "std"), target_arch = "arm"))]
use panic_halt as _;

#[cfg(all(not(feature = "std"), target_arch = "arm"))]
#[entry]
fn main() -> ! {
    // Attempt to allocate on the heap — should crash due to dummy allocator
    let _leaked = Box::leak(Box::new(5678u32));
    #[allow(clippy::empty_loop)]
    loop {}
}

// Generic no_std entry point for other architectures (like thumbv7em, xtensa, etc.)
#[cfg(all(
    not(feature = "std"),
    not(target_arch = "riscv32"),
    not(target_arch = "arm")
))]
extern crate alloc;
#[cfg(all(
    not(feature = "std"),
    not(target_arch = "riscv32"),
    not(target_arch = "arm")
))]
use alloc::boxed::Box;
#[cfg(all(
    not(feature = "std"),
    not(target_arch = "riscv32"),
    not(target_arch = "arm")
))]
use panic_halt as _;

#[cfg(all(
    not(feature = "std"),
    not(target_arch = "riscv32"),
    not(target_arch = "arm")
))]
#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    // Attempt to allocate on the heap — should crash due to dummy allocator
    let _leaked = Box::leak(Box::new(9999u32));
    // For architectures without a specific runtime, we loop indefinitely
    // The allocation attempt above should have already caused a panic
    #[allow(clippy::empty_loop)]
    loop {}
}

#[cfg(feature = "std")]
fn main() {
    println!(
        "This example is meant to be built and run for no_std targets to test heap allocation crashes."
    );
    println!(
        "Run with: cargo build --example heap_crash --target <no_std_target> --no-default-features"
    );
}
