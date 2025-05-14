#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(target_arch = "arm")]
mod cortex_m_logic {
    use cortex_m::asm;
    use cortex_m_rt::entry;
    use panic_halt as _; // Standard panic handler for Cortex-M // For nop

    // If we want to see output during size check (optional, adds size)
    // use cortex_m_semihosting::{debug, hprintln};

    use lit_bit_core::StateMachine;
    use lit_bit_core::core::{DefaultContext, MachineDefinition, Runtime, Transition};

    // --- State Definitions (can be very simple for size check) ---
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    enum LightState {
        Off,
        On,
    }

    // --- Event Definitions ---
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    enum LightEvent {
        Toggle,
    }

    // --- Context Definition ---
    type LightContext = DefaultContext; // Simplest context

    // --- Machine Definition (Static) ---
    const LIGHT_TRANSITIONS: &[Transition<LightState, LightEvent, LightContext>] = &[
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

    static LIGHT_MACHINE_DEF: MachineDefinition<LightState, LightEvent, LightContext> =
        MachineDefinition::new(LightState::Off, LIGHT_TRANSITIONS);

    #[entry]
    fn main_cortex_m_entry() -> ! {
        // Renamed to avoid conflict if main is defined outside
        let initial_context = DefaultContext::default();
        let mut runtime = Runtime::new(LIGHT_MACHINE_DEF.clone(), initial_context);

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
