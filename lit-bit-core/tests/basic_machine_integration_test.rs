// lit-bit-core/tests/basic_machine_integration_test.rs

#[cfg(test)]
pub mod basic_machine_integration_test {
    use core::convert::TryFrom;
    use heapless::String;
    use lit_bit_core::StateMachine;
    use lit_bit_macro::statechart;

    const ACTION_LOG_CAPACITY: usize = 20;
    const ACTION_STRING_CAPACITY: usize = 32;

    macro_rules! hstr {
        ($s:expr) => {
            String::<ACTION_STRING_CAPACITY>::try_from($s).expect("Failed to create test string")
        };
    }

    #[derive(Debug, Clone, PartialEq)]
    #[allow(clippy::struct_excessive_bools)]
    pub struct TestContext {
        count: i32,
        action_log: heapless::Vec<heapless::String<ACTION_STRING_CAPACITY>, ACTION_LOG_CAPACITY>,
    }

    impl Default for TestContext {
        fn default() -> Self {
            TestContext {
                count: 0,
                action_log: heapless::Vec::new(),
            }
        }
    }

    impl TestContext {
        fn record(&mut self, action_name: &str) {
            let s = heapless::String::try_from(action_name)
                .expect("Failed to create heapless string for action log");
            self.action_log.push(s).unwrap_or_else(|_val_not_pushed| {
                panic!(
                    "Action log overflow (capacity {ACTION_LOG_CAPACITY}). Could not log: {action_name}"
                );
            });
        }
        fn clear_log(&mut self) {
            self.action_log.clear();
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub enum TestEvent {
        #[default]
        Increment,
        Decrement,
        Reset,
        Forbidden,
    }

    fn entry_s1(context: &mut TestContext, _event: &TestEvent) {
        context.record("entry_s1");
    }

    fn exit_s1(context: &mut TestContext, _event: &TestEvent) {
        context.record("exit_s1");
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn transition_action_for_increment(context: &mut TestContext, _event: &TestEvent) {
        context.count += 1;
        context.record("action_increment");
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn guard_for_increment(context: &TestContext, _event: &TestEvent) -> bool {
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
        let expected_log_init: [String<ACTION_STRING_CAPACITY>; 1] = [hstr!("entry_s1")];
        assert_eq!(machine.context().action_log.as_slice(), &expected_log_init);

        machine.context_mut().clear_log();
        let transition_occurred_1 = machine.send(&TestEvent::Increment);
        assert!(
            transition_occurred_1,
            "Expected a transition for first Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        let expected_log_inc1: [String<ACTION_STRING_CAPACITY>; 2] =
            [hstr!("exit_s1"), hstr!("action_increment")];
        assert_eq!(machine.context().action_log.as_slice(), &expected_log_inc1);

        machine.context_mut().clear_log();
        let transition_occurred_reset = machine.send(&TestEvent::Reset);
        assert!(transition_occurred_reset, "Expected a transition for Reset");
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        let expected_log_reset1: [String<ACTION_STRING_CAPACITY>; 1] = [hstr!("entry_s1")];
        assert_eq!(
            machine.context().action_log.as_slice(),
            &expected_log_reset1
        );

        machine.context_mut().clear_log();
        let transition_occurred_inc2 = machine.send(&TestEvent::Increment);
        assert!(
            transition_occurred_inc2,
            "Expected a transition for second Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        let expected_log_inc2: [String<ACTION_STRING_CAPACITY>; 2] =
            [hstr!("exit_s1"), hstr!("action_increment")];
        assert_eq!(machine.context().action_log.as_slice(), &expected_log_inc2);

        machine.context_mut().clear_log();
        let transition_occurred_reset_2 = machine.send(&TestEvent::Reset);
        assert!(
            transition_occurred_reset_2,
            "Expected a transition for second Reset"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        let expected_log_reset2: [String<ACTION_STRING_CAPACITY>; 1] = [hstr!("entry_s1")];
        assert_eq!(
            machine.context().action_log.as_slice(),
            &expected_log_reset2
        );

        machine.context_mut().clear_log();
        let transition_occurred_blocked = machine.send(&TestEvent::Increment);
        assert!(
            !transition_occurred_blocked,
            "Expected no transition for blocked Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        assert!(
            machine.context().action_log.is_empty(),
            "No actions should occur for blocked increment"
        );
        assert_eq!(
            machine.context().count,
            2,
            "Count should remain 2 if transition is blocked"
        );

        machine.context_mut().clear_log();
        let transition_occurred_decrement = machine.send(&TestEvent::Decrement);
        assert!(
            transition_occurred_decrement,
            "Expected a transition for Decrement"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);

        let expected_decrement_log: [String<ACTION_STRING_CAPACITY>; 2] =
            [hstr!("exit_s1"), hstr!("entry_s1")];
        assert_eq!(
            machine.context().action_log.as_slice(),
            &expected_decrement_log,
            "Self-transition action order incorrect"
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
                .expect("Failed to create heapless string for ParallelInitContext log");
            let capacity = self.log.capacity(); // Get capacity before the move in push
            self.log.push(s).unwrap_or_else(|_val_not_pushed| {
                panic!(
                    "ParallelInitContext log overflow (capacity {capacity}). Could not log: {entry}"
                );
            });
        }
    }

    fn entry_p(ctx: &mut ParallelInitContext, _event: &TestEvent) {
        ctx.record("entry_p");
    }
    fn entry_r1(ctx: &mut ParallelInitContext, _event: &TestEvent) {
        ctx.record("entry_r1");
    }
    fn entry_r1a(ctx: &mut ParallelInitContext, _event: &TestEvent) {
        ctx.record("entry_r1a");
    }
    fn entry_r2(ctx: &mut ParallelInitContext, _event: &TestEvent) {
        ctx.record("entry_r2");
    }
    fn entry_r2x(ctx: &mut ParallelInitContext, _event: &TestEvent) {
        ctx.record("entry_r2x");
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
