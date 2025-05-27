#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

// Dummy allocator: satisfies linker for potential `alloc` references,
// but will crash if actual heap allocation occurs.
// This ensures heapless behavior at runtime.

#[cfg(target_arch = "riscv32")]
#[global_allocator]
static DUMMY: DummyAlloc = DummyAlloc;

#[cfg(target_arch = "riscv32")]
struct DummyAlloc;

#[cfg(target_arch = "riscv32")]
unsafe impl core::alloc::GlobalAlloc for DummyAlloc {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        // Panic immediately to prevent undefined behavior from null pointer dereference
        panic!("DummyAlloc: heap allocation attempted in no_std context")
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

// panic_halt is only used and needed for the riscv32 no_std target
#[cfg(target_arch = "riscv32")]
use panic_halt as _;

#[cfg(target_arch = "riscv32")]
mod riscv_logic {
    use riscv::asm;
    use riscv_rt::entry;

    use lit_bit_core::{
        ActionFn, MAX_ACTIVE_REGIONS, MachineDefinition, Runtime, SendResult, StateMachine,
        StateNode, Transition,
    };

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    #[repr(u8)]
    enum TrafficLightState {
        Red = 0,
        Green = 1,
        Yellow = 2,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    #[repr(u8)]
    enum TrafficLightEvent {
        TimerElapsed = 0,
    }

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    struct TrafficLightContext {
        cycle_count: u32,
    }

    unsafe fn uart_putc(c: u8) {
        const UART_BASE: *mut u8 = 0x1000_0000 as *mut u8;
        unsafe {
            core::ptr::write_volatile(UART_BASE, c);
        }
    }

    unsafe fn uart_print_str(s: &str) {
        for byte in s.bytes() {
            unsafe {
                uart_putc(byte);
            }
        }
    }

    fn log_red(_context: &mut TrafficLightContext, _event: &TrafficLightEvent) {
        unsafe {
            uart_print_str("UART: Light is now RED.\n");
        }
    }

    fn log_green(_context: &mut TrafficLightContext, _event: &TrafficLightEvent) {
        unsafe {
            uart_print_str("UART: Light is now GREEN.\n");
        }
    }

    fn log_yellow(_context: &mut TrafficLightContext, _event: &TrafficLightEvent) {
        unsafe {
            uart_print_str("UART: Light is now YELLOW.\n");
        }
    }

    fn increment_cycle(context: &mut TrafficLightContext, _event: &TrafficLightEvent) {
        context.cycle_count += 1;
    }

    fn matches_timer_elapsed(event: &TrafficLightEvent) -> bool {
        matches!(event, TrafficLightEvent::TimerElapsed)
    }

    const TRAFFIC_LIGHT_TRANSITIONS: &[Transition<
        TrafficLightState,
        TrafficLightEvent,
        TrafficLightContext,
    >] = &[
        Transition {
            from_state: TrafficLightState::Red,
            to_state: TrafficLightState::Green,
            action: None,
            guard: None,
            match_fn: Some(matches_timer_elapsed),
        },
        Transition {
            from_state: TrafficLightState::Green,
            to_state: TrafficLightState::Yellow,
            action: None,
            guard: None,
            match_fn: Some(matches_timer_elapsed),
        },
        Transition {
            from_state: TrafficLightState::Yellow,
            to_state: TrafficLightState::Red,
            action: Some(increment_cycle as ActionFn<TrafficLightContext, TrafficLightEvent>),
            guard: None,
            match_fn: Some(matches_timer_elapsed),
        },
    ];

    const TRAFFIC_LIGHT_STATENODES: &[StateNode<
        TrafficLightState,
        TrafficLightContext,
        TrafficLightEvent,
    >] = &[
        StateNode {
            id: TrafficLightState::Red,
            parent: None,
            initial_child: None,
            entry_action: Some(log_red as ActionFn<TrafficLightContext, TrafficLightEvent>),
            exit_action: None,
            is_parallel: false,
        },
        StateNode {
            id: TrafficLightState::Green,
            parent: None,
            initial_child: None,
            entry_action: Some(log_green as ActionFn<TrafficLightContext, TrafficLightEvent>),
            exit_action: None,
            is_parallel: false,
        },
        StateNode {
            id: TrafficLightState::Yellow,
            parent: None,
            initial_child: None,
            entry_action: Some(log_yellow as ActionFn<TrafficLightContext, TrafficLightEvent>),
            exit_action: None,
            is_parallel: false,
        },
    ];

    const TRAFFIC_LIGHT_MACHINE_DEF: MachineDefinition<
        TrafficLightState,
        TrafficLightEvent,
        TrafficLightContext,
    > = MachineDefinition::new(
        TRAFFIC_LIGHT_STATENODES,
        TRAFFIC_LIGHT_TRANSITIONS,
        TrafficLightState::Red,
    );

    const M: usize = 2;
    const MAX_NODES_CALC: usize = M * MAX_ACTIVE_REGIONS;

    type TrafficLightRuntime = Runtime<
        TrafficLightState,
        TrafficLightEvent,
        TrafficLightContext,
        M,
        MAX_ACTIVE_REGIONS,
        MAX_NODES_CALC,
    >;

    #[entry]
    fn main_riscv_entry() -> ! {
        unsafe {
            uart_print_str("UART: Entered main_riscv!\n");
            uart_print_str("UART: Starting traffic light simulation...\n");
        }

        let initial_context = TrafficLightContext { cycle_count: 0 };
        let initial_event = TrafficLightEvent::TimerElapsed;

        let mut runtime: TrafficLightRuntime =
            Runtime::new(&TRAFFIC_LIGHT_MACHINE_DEF, initial_context, &initial_event)
                .expect("Failed to create traffic light state machine");

        unsafe {
            uart_print_str("UART: State machine created.\n");
        }

        for _ in 0..7 {
            unsafe {
                uart_print_str("UART: Event -> ");
            }
            match runtime.send(&TrafficLightEvent::TimerElapsed) {
                SendResult::Transitioned => unsafe {
                    uart_print_str("UART: Transitioned.\n");
                },
                SendResult::NoMatch => unsafe {
                    uart_print_str("UART: No Transition.\n");
                },
                SendResult::Error(_) => unsafe {
                    uart_print_str("UART: ERROR during transition!\n");
                },
            }
        }

        unsafe {
            uart_print_str("UART: Simulation finished.\n");
        }

        loop {
            asm::nop();
        }
    }
}

// Dummy main for non-riscv32 targets.
// This will be a std-linking program when checked by clippy for host.
#[cfg(not(target_arch = "riscv32"))]
fn main() {
    println!("This traffic_light example is intended for target_arch = \"riscv32\".");
}
