// lit-bit-core/tests/basic_machine_integration_test.rs

#[cfg(test)]
mod basic_machine_integration_test {
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
        // ... other assertions ...
        machine.context_mut().entry_action_called = false;
        let transition_occurred_1 = machine.send(TestEvent::Increment);
        assert!(
            transition_occurred_1,
            "Expected a transition for first Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        // ... other assertions ...
        machine.context_mut().exit_action_called = false;
        machine.context_mut().transition_action_called = false;
        let transition_occurred_reset = machine.send(TestEvent::Reset);
        assert!(transition_occurred_reset, "Expected a transition for Reset");
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        // ... other assertions ...
        machine.context_mut().entry_action_called = false;
        machine.context_mut().exit_action_called = false;
        let transition_occurred_inc2 = machine.send(TestEvent::Increment);
        assert!(
            transition_occurred_inc2,
            "Expected a transition for second Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        // ... other assertions ...
        machine.context_mut().exit_action_called = false;
        machine.context_mut().transition_action_called = false;
        machine.context_mut().entry_action_called = false;
        let transition_occurred_reset_2 = machine.send(TestEvent::Reset);
        assert!(
            transition_occurred_reset_2,
            "Expected a transition for second Reset"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        // ... other assertions ...
        machine.context_mut().entry_action_called = false;
        machine.context_mut().exit_action_called = false;
        let transition_occurred_blocked = machine.send(TestEvent::Increment);
        assert!(
            !transition_occurred_blocked,
            "Expected no transition for blocked Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        // ... other assertions ...
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
    use lit_bit_core::StateMachine;
    use lit_bit_macro::statechart;
    // Reuse TestEvent from the other module for simplicity as no events are sent.
    use crate::basic_machine_integration_test::TestEvent;

    #[derive(Debug, Default, Clone, PartialEq)]
    pub struct ParallelInitContext {
        log: heapless::Vec<heapless::String<64>, 16>,
    }

    impl ParallelInitContext {
        fn record(&mut self, entry: &str) {
            let mut s = heapless::String::new();
            assert!(
                s.push_str(entry).is_ok(),
                "Failed to record log entry: {entry}"
            );
            assert!(
                self.log.push(s).is_ok(),
                "Log vec full, cannot record: {entry}"
            );
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
                state R1A {
                    entry: entry_r1a;
                }
            }

            state R2 {
                initial: R2X;
                entry: entry_r2;
                state R2X {
                    entry: entry_r2x;
                }
                state R2Y {}
            }
        }
        state Other {}
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_initial_parallel_state_activation() {
        let machine = ParallelInitialMachine::new(ParallelInitContext::default());

        let active_states = machine.state();

        assert_eq!(
            active_states.len(),
            2,
            "Should have two active leaf states. Got: {active_states:?}"
        );
        let is_r1a_active = active_states
            .iter()
            .any(|s| *s == ParallelInitialMachineStateId::PR1R1A);
        let is_r2x_active = active_states
            .iter()
            .any(|s| *s == ParallelInitialMachineStateId::PR2R2X);
        assert!(
            is_r1a_active,
            "Active state PR1R1A missing. Active: {active_states:?}"
        );
        assert!(
            is_r2x_active,
            "Active state PR2R2X missing. Active: {active_states:?}"
        );

        let log = machine.context().log.as_slice();
        assert_eq!(
            log.len(),
            5,
            "Incorrect number of entry actions logged. Log: {log:?}"
        );
        assert_eq!(
            log[0].as_str(),
            "EnterP",
            "First log should be EnterP. Log: {log:?}"
        );

        let expected_entries = ["EnterP", "EnterR1", "EnterR1A", "EnterR2", "EnterR2X"];
        for entry_val in expected_entries {
            assert!(
                log.iter().any(|s| s.as_str() == entry_val),
                "Missing log entry: {entry_val}. Log: {log:?}"
            );
        }

        let pos = |entry_val: &str| log.iter().position(|s| s.as_str() == entry_val);
        if let (Some(p_idx), Some(r1_idx), Some(r1a_idx), Some(r2_idx), Some(r2x_idx)) = (
            pos("EnterP"),
            pos("EnterR1"),
            pos("EnterR1A"),
            pos("EnterR2"),
            pos("EnterR2X"),
        ) {
            assert!(
                p_idx < r1_idx,
                "P entry (idx {p_idx}) should be before R1 entry (idx {r1_idx}). Log: {log:?}"
            );
            assert!(
                p_idx < r2_idx,
                "P entry (idx {p_idx}) should be before R2 entry (idx {r2_idx}). Log: {log:?}"
            );
            assert!(
                r1_idx < r1a_idx,
                "R1 entry (idx {r1_idx}) should be before R1A entry (idx {r1a_idx}). Log: {log:?}"
            );
            assert!(
                r2_idx < r2x_idx,
                "R2 entry (idx {r2_idx}) should be before R2X entry (idx {r2x_idx}). Log: {log:?}"
            );
        } else {
            panic!("One or more expected log entries for order checking are missing. Log: {log:?}");
        }
    }
}
