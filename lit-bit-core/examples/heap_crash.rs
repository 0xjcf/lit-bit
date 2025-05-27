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
    /// Panics immediately if a heap allocation is attempted in a no_std context.
    ///
    /// This method is part of a dummy global allocator and always panics to prevent
    /// undefined behavior from null pointer dereferences when heap allocation is not supported.
    ///
    /// # Panics
    ///
    /// Always panics with a message indicating that heap allocation was attempted in a no_std context.
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        // Panic immediately to prevent undefined behavior from null pointer dereference
        panic!("DummyAlloc: heap allocation attempted in no_std context")
    }
    /// Does nothing when a heap allocation is deallocated.
///
/// This dummy deallocator is used in no_std environments to prevent undefined behavior from heap deallocation attempts when no real allocator is present.
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
/// Entry point for RISC-V32 no_std targets that triggers a panic on heap allocation.
///
/// Attempts to allocate a boxed integer on the heap, which causes an immediate panic due to the dummy global allocator. Enters an infinite loop after the allocation attempt. Intended to demonstrate controlled heap allocation failure in embedded environments.
fn main() -> ! {
    // Attempt to allocate on the heap — should crash due to dummy allocator
    let _leaked = Box::leak(Box::new(1234u32));
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
/// Entry point for RISC-V32 no_std targets that triggers a panic on heap allocation.
///
/// Attempts to allocate a boxed integer on the heap, which causes an immediate panic due to the dummy global allocator. Enters an infinite loop after the allocation attempt. Intended to demonstrate and test heap allocation failure in embedded environments.
fn main() -> ! {
    // Attempt to allocate on the heap — should crash due to dummy allocator
    let _leaked = Box::leak(Box::new(5678u32));
    loop {}
}

// Generic no_std main for other architectures (like thumbv7em, xtensa, etc.)
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
/// Attempts a heap allocation to demonstrate a controlled panic with the dummy allocator on generic `no_std` targets.
///
/// This function leaks a boxed integer, which triggers a panic from the dummy global allocator, simulating a heap allocation failure in environments without a real allocator. Intended for use on `no_std` targets that do not require a specific runtime entry point.
fn main() {
    // Attempt to allocate on the heap — should crash due to dummy allocator
    let _leaked = Box::leak(Box::new(9999u32));
    // For architectures without a specific runtime, we just return
    // The allocation attempt above should have already caused a panic
}

#[cfg(feature = "std")]
/// Prints instructions for building and running the example on `no_std` targets.
///
/// This function is intended for use when the standard library is enabled, providing guidance on how to test heap allocation crashes in `no_std` environments.
fn main() {
    println!(
        "This example is meant to be built and run for no_std targets to test heap allocation crashes."
    );
    println!(
        "Run with: cargo build --example heap_crash --target <no_std_target> --no-default-features"
    );
}
