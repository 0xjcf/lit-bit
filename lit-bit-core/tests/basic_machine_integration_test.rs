// lit-bit-core/tests/basic_machine_integration_test.rs

#[cfg(test)]
pub mod basic_machine_integration_test {
    use lit_bit_core::StateMachine;
    use lit_bit_macro::statechart;
    // heapless::Vec might be used by tests if they assert on machine.state()
    // For this module, the existing assertions use .as_slice() which doesn't directly require Vec in this scope

    #[derive(Debug, Default, Clone, PartialEq)]
    #[allow(clippy::struct_excessive_bools)]
    pub struct TestContext {
        count: i32,
        entry_action_called: bool,
        exit_action_called: bool,
        transition_action_called: bool,
        guard_called_for_increment: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum TestEvent {
        Increment,
        Decrement,
        Reset,
        Forbidden,
    }

    fn entry_s1(context: &mut TestContext) {
        context.entry_action_called = true;
    }

    fn exit_s1(context: &mut TestContext) {
        context.exit_action_called = true;
    }

    fn transition_action_for_increment(context: &mut TestContext) {
        context.count += 1;
        context.transition_action_called = true;
    }

    fn guard_for_increment(context: &TestContext, _event: TestEvent) -> bool {
        context.count < 2
    }

    statechart! {
        name: TestMachine,
        context: TestContext,
        event: TestEvent,
        initial: State1,
        state State1 {
            entry: entry_s1;
            exit: exit_s1;
            on Increment [guard guard_for_increment] => State2 [action transition_action_for_increment];
            on Decrement => State1;
        }
        state State2 {
            on Reset => State1;
            on Forbidden => State2;
        }
    }

    #[test]
    fn test_basic_state_machine_transitions_and_actions() {
        let mut machine = TestMachine::new(TestContext::default());
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        assert!(
            machine.context().entry_action_called,
            "State1 entry action should have been called on init"
        );
        machine.context_mut().entry_action_called = false;
        let transition_occurred_1 = machine.send(TestEvent::Increment);
        assert!(
            transition_occurred_1,
            "Expected a transition for first Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        machine.context_mut().exit_action_called = false;
        machine.context_mut().transition_action_called = false;
        let transition_occurred_reset = machine.send(TestEvent::Reset);
        assert!(transition_occurred_reset, "Expected a transition for Reset");
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        machine.context_mut().entry_action_called = false;
        machine.context_mut().exit_action_called = false;
        let transition_occurred_inc2 = machine.send(TestEvent::Increment);
        assert!(
            transition_occurred_inc2,
            "Expected a transition for second Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        machine.context_mut().exit_action_called = false;
        machine.context_mut().transition_action_called = false;
        machine.context_mut().entry_action_called = false;
        let transition_occurred_reset_2 = machine.send(TestEvent::Reset);
        assert!(
            transition_occurred_reset_2,
            "Expected a transition for second Reset"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        machine.context_mut().entry_action_called = false;
        machine.context_mut().exit_action_called = false;
        let transition_occurred_blocked = machine.send(TestEvent::Increment);
        assert!(
            !transition_occurred_blocked,
            "Expected no transition for blocked Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        assert_eq!(
            machine.context().count,
            2,
            "Count should remain 2 if transition is blocked"
        );
        machine.context_mut().entry_action_called = false;
        machine.context_mut().exit_action_called = false;
        machine.context_mut().transition_action_called = false;

        let transition_occurred_decrement = machine.send(TestEvent::Decrement);
        assert!(
            transition_occurred_decrement,
            "Expected a transition for Decrement"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);

        assert!(
            machine.context().exit_action_called,
            "State1 exit action should be called on self-transition via Decrement"
        );
        assert!(
            machine.context().entry_action_called,
            "State1 entry action should be called on self-transition via Decrement"
        );
        assert!(
            !machine.context().transition_action_called,
            "Decrement transition should not have a specific transition action"
        );
    }
}

// --- Test for Parallel Initial State Activation ---
#[cfg(test)]
mod parallel_initial_state_test {
    use core::convert::TryFrom;
    use lit_bit_core::StateMachine;
    use lit_bit_macro::statechart;
    // Reuse TestEvent from the other module for simplicity as no events are sent.
    use super::basic_machine_integration_test::TestEvent;

    #[derive(Debug, Default, Clone, PartialEq)]
    pub struct ParallelInitContext {
        log: heapless::Vec<heapless::String<32>, 10>,
    }

    impl ParallelInitContext {
        fn record(&mut self, entry: &str) {
            let s = heapless::String::try_from(entry)
                .expect("Failed to create heapless string for log entry");
            self.log
                .push(s)
                .expect("Log vec full in ParallelInitContext");
        }
    }

    fn entry_p(ctx: &mut ParallelInitContext) {
        ctx.record("EnterP");
    }
    fn entry_r1(ctx: &mut ParallelInitContext) {
        ctx.record("EnterR1");
    }
    fn entry_r1a(ctx: &mut ParallelInitContext) {
        ctx.record("EnterR1A");
    }
    fn entry_r2(ctx: &mut ParallelInitContext) {
        ctx.record("EnterR2");
    }
    fn entry_r2x(ctx: &mut ParallelInitContext) {
        ctx.record("EnterR2X");
    }

    statechart! {
        name: ParallelInitialMachine,
        context: ParallelInitContext,
        event: TestEvent,
        initial: P,
        state P [parallel] {
            entry: entry_p;
            state R1 {
                initial: R1A;
                entry: entry_r1;
                state R1A { entry: entry_r1a; /* on E_TOGGLE => R1B; */ }
                state R1B { /* entry: entry_r1b; */ }
            }
            state R2 {
                initial: R2X;
                entry: entry_r2;
                state R2X { entry: entry_r2x; /* on E_TOGGLE => R2Y; */ }
                state R2Y { /* entry: entry_r2y; */ }
            }
        }
        state Other {}
    }

    #[test]
    fn test_initial_parallel_state_activation() {
        let machine = ParallelInitialMachine::new(ParallelInitContext::default());

        let active_states = machine.state();

        assert_eq!(
            active_states.len(),
            2,
            "Should have two active leaf states. Got len: {} expected: 2. Full active states: {:?}",
            active_states.len(),
            active_states
        );

        // Use from_str_path for assertions
        let expected_state_in_region1 = ParallelInitialMachineStateId::from_str_path("P_R1_R1A")
            .expect("State P_R1_R1A not found via from_str_path");
        let expected_state_in_region2 = ParallelInitialMachineStateId::from_str_path("P_R2_R2X")
            .expect("State P_R2_R2X not found via from_str_path");

        assert!(
            active_states.contains(&expected_state_in_region1),
            "Active state P_R1_R1A missing. Active: {active_states:?}"
        );
        assert!(
            active_states.contains(&expected_state_in_region2),
            "Active state P_R2_R2X missing. Active: {active_states:?}"
        );

        // Keep original log assertions but assert the whole sequence
        let log = machine.context().log.as_slice();
        let actual_log_strs: Vec<&str> = log.iter().map(heapless::String::as_str).collect();

        let expected_log_sequence = vec!["EnterP", "EnterR1", "EnterR1A", "EnterR2", "EnterR2X"];
        assert_eq!(
            actual_log_strs, expected_log_sequence,
            "Full log sequence mismatch."
        );
    }
}
