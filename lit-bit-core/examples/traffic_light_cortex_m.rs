#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(target_arch = "arm")]
mod cortex_m_logic {
    use cortex_m::asm;
    use cortex_m_rt::entry;
    use panic_halt as _; // Standard panic handler for Cortex-M // For nop

    // If we want to see output during size check (optional, adds size)
    // use cortex_m_semihosting::{debug, hprintln};

    use lit_bit_core::{
        DefaultContext,
        MAX_ACTIVE_REGIONS, // Use re-exported version
        MachineDefinition,
        Runtime,
        StateNode,
        Transition,
    };

    // Define states for the traffic light
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    enum LightState {
        Off,
        On,
    }

    // Define events for the traffic light
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    enum LightEvent {
        Toggle,
    }

    // For this example, we'll use the DefaultContext since actions don't modify a specific context.
    type BlinkyContext = DefaultContext;

    const LIGHT_TRANSITIONS: &[Transition<LightState, LightEvent, BlinkyContext>] = &[
        Transition {
            from_state: LightState::Off,
            event: LightEvent::Toggle,
            to_state: LightState::On,
            action: None,
            guard: None,
        },
        Transition {
            from_state: LightState::On,
            event: LightEvent::Toggle,
            to_state: LightState::Off,
            action: None,
            guard: None,
        },
    ];

    // Define the states (even if simple, the definition needs an array)
    const LIGHT_STATENODES: &[StateNode<LightState, BlinkyContext>] = &[];

    #[allow(dead_code)]
    const BLINKY_MACHINE_DEF: MachineDefinition<LightState, LightEvent, BlinkyContext> =
        MachineDefinition::new(
            LIGHT_STATENODES, // Added states argument
            LIGHT_TRANSITIONS,
            LightState::Off, // Initial state
        );

    // Define M and MAX_NODES_FOR_COMPUTATION for the Runtime instantiation
    // M: Represents the maximum expected hierarchy depth of any state path in this specific machine.
    // For this simple blinky machine, depth is 1 (only top-level states). Value 2 provides a small buffer.
    const M: usize = 2;
    // MAX_NODES_FOR_COMPUTATION: Buffer for computations involving multiple hierarchy branches.
    // Calculated as M * MAX_ACTIVE_REGIONS.
    const MAX_NODES_CALC: usize = M * MAX_ACTIVE_REGIONS;

    #[entry]
    fn main_cortex_m_entry() -> ! {
        let initial_context = DefaultContext::default();
        // Use turbofish for const generics, allowing type inference for State, Event, Context
        let mut runtime =
            Runtime::<_, _, _, M, MAX_NODES_CALC>::new(&BLINKY_MACHINE_DEF, initial_context);

        runtime.send(LightEvent::Toggle);
        runtime.send(LightEvent::Toggle);
        let _ = runtime.state();
        let _ = runtime.context();

        loop {
            asm::nop();
        }
    }
}

// Dummy main for non-ARM targets.
#[cfg(not(target_arch = "arm"))]
fn main() {
    println!("This traffic_light_cortex_m example is intended for target_arch = \"arm\".");
}
