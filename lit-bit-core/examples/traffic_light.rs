#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

// panic_halt is only used and needed for the riscv32 no_std target
#[cfg(target_arch = "riscv32")]
use panic_halt as _;

#[cfg(target_arch = "riscv32")]
mod riscv_logic {
    use riscv::asm;
    use riscv_rt::entry;
    use semihosting::println;

    use lit_bit_core::{
        StateMachine,
        core::{
            ActionFn, MAX_ACTIVE_REGIONS, MachineDefinition, Runtime, SendResult, StateNode,
            Transition,
        },
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

    // Reinstate TrafficLightContext with cycle_count
    #[derive(Debug, Clone, Default, PartialEq, Eq)] // Added Default for initialization
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
            uart_print_str("U: Light is now RED.\n");
        }
    }

    fn log_green(_context: &mut TrafficLightContext, _event: &TrafficLightEvent) {
        unsafe {
            uart_print_str("U: Light is now GREEN.\n");
        }
    }

    fn log_yellow(_context: &mut TrafficLightContext, _event: &TrafficLightEvent) {
        unsafe {
            uart_print_str("U: Light is now YELLOW.\n");
        }
    }

    fn increment_cycle(context: &mut TrafficLightContext, _event: &TrafficLightEvent) {
        context.cycle_count += 1;
    }

    // Match function for timer elapsed event
    fn matches_timer_elapsed(event: &TrafficLightEvent) -> bool {
        matches!(event, TrafficLightEvent::TimerElapsed)
    }

    // Define the transitions
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

    // Define the states (even if simple, the definition needs an array)
    #[allow(dead_code)] // Suppress dead code warning as it's used via TRAFFIC_LIGHT_MACHINE_DEF
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

    // Create the machine definition
    // This is what the `statechart!` macro would generate.
    const TRAFFIC_LIGHT_MACHINE_DEF: MachineDefinition<
        TrafficLightState,
        TrafficLightEvent,
        TrafficLightContext,
    > = MachineDefinition::new(
        TRAFFIC_LIGHT_STATENODES, // Added states argument
        TRAFFIC_LIGHT_TRANSITIONS,
        TrafficLightState::Red, // Initial state
    );

    const M: usize = 2; // Max hierarchy depth for this simple machine (flat = 1, 2 is safe)
    const MAX_NODES_CALC: usize = M * MAX_ACTIVE_REGIONS;

    // Type alias for this specific Runtime configuration
    type TrafficLightRuntime =
        Runtime<TrafficLightState, TrafficLightEvent, TrafficLightContext, M, MAX_NODES_CALC>;

    #[entry]
    fn main_riscv_entry() -> ! {
        unsafe {
            uart_print_str("UART: Entered main_riscv!\n");
        }
        println!("SEMI: Entered main_riscv! Semihosting test.");
        println!("SEMI: Starting traffic light simulation...");
        unsafe {
            uart_print_str("UART: Starting simulation...\n");
        }

        let initial_context = TrafficLightContext { cycle_count: 0 };
        let initial_event = TrafficLightEvent::TimerElapsed; // Initial event
        // Use the type alias
        let mut runtime: TrafficLightRuntime =
            Runtime::new(&TRAFFIC_LIGHT_MACHINE_DEF, initial_context, &initial_event)
                .expect("Failed to create traffic light state machine");

        unsafe {
            uart_print_str("UART: SM created.\n");
        }

        for _ in 0..7 {
            unsafe {
                uart_print_str("\nUART: Event -> ");
            }
            match runtime.send(&TrafficLightEvent::TimerElapsed) {
                SendResult::Transitioned => unsafe {
                    uart_print_str("UART: Transitioned.\n");
                },
                SendResult::NoMatch => unsafe {
                    uart_print_str("UART: No Transition.\n");
                },
                SendResult::Error(e) => {
                    println!("SEMI: Runtime error: {e:?}");
                    unsafe {
                        uart_print_str("UART: ERROR during transition!\n");
                    }
                }
            }
        }

        println!("\nSEMI: Simulation finished.");
        unsafe {
            uart_print_str("\nUART: Simulation finished.\n");
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
