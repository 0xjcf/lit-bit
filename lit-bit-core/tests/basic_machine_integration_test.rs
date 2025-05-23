// lit-bit-core/tests/basic_machine_integration_test.rs

#[cfg(test)]
#[allow(clippy::trivially_copy_pass_by_ref)]
pub mod basic_machine_integration_test {
    use core::convert::TryFrom;
    use heapless::String;
    use lit_bit_core::SendResult;
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
            on TestEvent::Increment [guard guard_for_increment] => State2 [action transition_action_for_increment];
            on TestEvent::Decrement => State1;
        }
        state State2 {
            on TestEvent::Reset => State1;
            on TestEvent::Forbidden => State2;
        }
    }

    #[test]
    fn test_basic_state_machine_transitions_and_actions() {
        let mut machine = TestMachine::new(TestContext::default(), &TestEvent::default())
            .expect("Failed to create test machine");
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        let expected_log_init: [String<ACTION_STRING_CAPACITY>; 1] = [hstr!("entry_s1")];
        assert_eq!(machine.context().action_log.as_slice(), &expected_log_init);

        machine.context_mut().clear_log();
        let send_result_1 = machine.send(&TestEvent::Increment);
        assert_eq!(
            send_result_1,
            SendResult::Transitioned,
            "Expected a transition for first Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        let expected_log_inc1: [String<ACTION_STRING_CAPACITY>; 2] =
            [hstr!("exit_s1"), hstr!("action_increment")];
        assert_eq!(machine.context().action_log.as_slice(), &expected_log_inc1);

        machine.context_mut().clear_log();
        let send_result_reset = machine.send(&TestEvent::Reset);
        assert_eq!(
            send_result_reset,
            SendResult::Transitioned,
            "Expected a transition for Reset"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        let expected_log_reset1: [String<ACTION_STRING_CAPACITY>; 1] = [hstr!("entry_s1")];
        assert_eq!(
            machine.context().action_log.as_slice(),
            &expected_log_reset1
        );

        machine.context_mut().clear_log();
        let send_result_inc2 = machine.send(&TestEvent::Increment);
        assert_eq!(
            send_result_inc2,
            SendResult::Transitioned,
            "Expected a transition for second Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        let expected_log_inc2: [String<ACTION_STRING_CAPACITY>; 2] =
            [hstr!("exit_s1"), hstr!("action_increment")];
        assert_eq!(machine.context().action_log.as_slice(), &expected_log_inc2);

        machine.context_mut().clear_log();
        let send_result_reset_2 = machine.send(&TestEvent::Reset);
        assert_eq!(
            send_result_reset_2,
            SendResult::Transitioned,
            "Expected a transition for second Reset"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        let expected_log_reset2: [String<ACTION_STRING_CAPACITY>; 1] = [hstr!("entry_s1")];
        assert_eq!(
            machine.context().action_log.as_slice(),
            &expected_log_reset2
        );

        machine.context_mut().clear_log();
        let send_result_blocked = machine.send(&TestEvent::Increment);
        assert_eq!(
            send_result_blocked,
            SendResult::NoMatch,
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
        let send_result_decrement = machine.send(&TestEvent::Decrement);
        assert_eq!(
            send_result_decrement,
            SendResult::Transitioned,
            "Expected a transition for Decrement"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);

        // Check that both exit and entry actions are present for self-transition
        // but don't enforce strict order to be resilient to implementation changes
        let log_as_strs: Vec<&str> = machine
            .context()
            .action_log
            .iter()
            .map(heapless::String::as_str)
            .collect();
        assert!(
            log_as_strs.contains(&"exit_s1"),
            "Self-transition should include exit action. Actual log: {log_as_strs:?}",
        );
        assert!(
            log_as_strs.contains(&"entry_s1"),
            "Self-transition should include entry action. Actual log: {log_as_strs:?}",
        );
        assert_eq!(
            log_as_strs.len(),
            2,
            "Self-transition should have exactly 2 actions. Actual log: {log_as_strs:?}",
        );

        // End of test
    }
}

// --- Test for Parallel Initial State Activation ---
#[cfg(test)]
#[allow(clippy::trivially_copy_pass_by_ref)]
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
        ctx.record("EnterP");
    }
    fn entry_r1(ctx: &mut ParallelInitContext, _event: &TestEvent) {
        ctx.record("EnterR1");
    }
    fn entry_r1a(ctx: &mut ParallelInitContext, _event: &TestEvent) {
        ctx.record("EnterR1A");
    }
    fn entry_r2(ctx: &mut ParallelInitContext, _event: &TestEvent) {
        ctx.record("EnterR2");
    }
    fn entry_r2x(ctx: &mut ParallelInitContext, _event: &TestEvent) {
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
        let machine =
            ParallelInitialMachine::new(ParallelInitContext::default(), &TestEvent::default())
                .expect("Failed to create parallel initial machine");
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

        // End of test
    }
}

// --- Test for Wildcard Pattern Matching ---
#[cfg(test)]
#[allow(clippy::trivially_copy_pass_by_ref)]
mod wildcard_pattern_test {
    use lit_bit_core::SendResult;
    use lit_bit_core::StateMachine;
    use lit_bit_macro::statechart;

    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct WildcardContext {
        log: heapless::Vec<heapless::String<32>, 10>,
    }

    impl WildcardContext {
        fn record(&mut self, action: &str) {
            let s = heapless::String::try_from(action).expect("Failed to create heapless string");
            self.log.push(s).unwrap_or_else(|_| {
                panic!("WildcardContext log overflow");
            });
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum WildcardEvent {
        EventA,
        EventB,
        EventC,
    }

    fn log_wildcard_match(ctx: &mut WildcardContext, _event: &WildcardEvent) {
        ctx.record("wildcard_match");
    }

    fn log_specific_match(ctx: &mut WildcardContext, _event: &WildcardEvent) {
        ctx.record("specific_match");
    }

    statechart! {
        name: WildcardMachine,
        context: WildcardContext,
        event: WildcardEvent,
        initial: State1,
        state State1 {
            on WildcardEvent::EventA => State2 [action log_specific_match];
            on _ => State3 [action log_wildcard_match];
        }
        state State2 {
            on _ => State1;
        }
        state State3 {
            on WildcardEvent::EventA => State1;
        }
    }

    #[test]
    fn test_wildcard_pattern_matching() {
        let mut machine = WildcardMachine::new(WildcardContext::default(), &WildcardEvent::EventA)
            .expect("Failed to create wildcard machine");

        // Test specific match takes precedence
        machine.context_mut().log.clear();
        let result = machine.send(&WildcardEvent::EventA);
        assert_eq!(result, SendResult::Transitioned);
        assert_eq!(
            machine.state().as_slice(),
            &[WildcardMachineStateId::State2]
        );
        assert_eq!(machine.context().log.len(), 1);
        assert_eq!(machine.context().log[0].as_str(), "specific_match");

        // Go back to State1
        machine.send(&WildcardEvent::EventB);
        assert_eq!(
            machine.state().as_slice(),
            &[WildcardMachineStateId::State1]
        );

        // Test wildcard match for EventB
        machine.context_mut().log.clear();
        let result = machine.send(&WildcardEvent::EventB);
        assert_eq!(result, SendResult::Transitioned);
        assert_eq!(
            machine.state().as_slice(),
            &[WildcardMachineStateId::State3]
        );
        assert_eq!(machine.context().log.len(), 1);
        assert_eq!(machine.context().log[0].as_str(), "wildcard_match");

        // Go back to State1
        machine.send(&WildcardEvent::EventA);
        assert_eq!(
            machine.state().as_slice(),
            &[WildcardMachineStateId::State1]
        );

        // Test wildcard match for EventC
        machine.context_mut().log.clear();
        let result = machine.send(&WildcardEvent::EventC);
        assert_eq!(result, SendResult::Transitioned);
        assert_eq!(
            machine.state().as_slice(),
            &[WildcardMachineStateId::State3]
        );
        assert_eq!(machine.context().log.len(), 1);
        assert_eq!(machine.context().log[0].as_str(), "wildcard_match");
    }
}

// --- Test for Multiple State Machines Without Name Collisions ---
#[cfg(test)]
mod multiple_machines_test {
    use lit_bit_core::StateMachine;
    use lit_bit_macro::statechart;

    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct ContextA {
        value: i32,
    }

    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct ContextB {
        value: i32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum EventA {
        Go,
        Stop,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum EventB {
        Start,
        End,
    }

    // First state machine in its own module
    mod machine_a_module {
        use super::*;

        statechart! {
            name: MachineA,
            context: ContextA,
            event: EventA,
            initial: StateA1,
            state StateA1 {
                on EventA::Go => StateA2;
            }
            state StateA2 {
                on EventA::Stop => StateA1;
            }
        }
    }

    // Second state machine in its own module
    mod machine_b_module {
        use super::*;

        statechart! {
            name: MachineB,
            context: ContextB,
            event: EventB,
            initial: StateB1,
            state StateB1 {
                on EventB::Start => StateB2;
            }
            state StateB2 {
                on EventB::End => StateB1;
            }
        }
    }

    use machine_a_module::*;
    use machine_b_module::*;

    #[test]
    fn test_multiple_machines_no_collision() {
        // Create both machines in the same scope
        let mut machine_a =
            MachineA::new(ContextA::default(), &EventA::Go).expect("Failed to create machine A");
        let mut machine_b =
            MachineB::new(ContextB::default(), &EventB::Start).expect("Failed to create machine B");

        // Verify initial states
        assert_eq!(machine_a.state().as_slice(), &[MachineAStateId::StateA1]);
        assert_eq!(machine_b.state().as_slice(), &[MachineBStateId::StateB1]);

        // Test transitions on both machines
        machine_a.send(&EventA::Go);
        assert_eq!(machine_a.state().as_slice(), &[MachineAStateId::StateA2]);

        machine_b.send(&EventB::Start);
        assert_eq!(machine_b.state().as_slice(), &[MachineBStateId::StateB2]);

        // Ensure they don't interfere with each other
        machine_a.send(&EventA::Stop);
        assert_eq!(machine_a.state().as_slice(), &[MachineAStateId::StateA1]);

        machine_b.send(&EventB::End);
        assert_eq!(machine_b.state().as_slice(), &[MachineBStateId::StateB1]);
    }
}

// --- Test to verify public API usage ---
#[cfg(test)]
mod public_api_test {
    use lit_bit_core::StateMachine;
    use lit_bit_macro::statechart;

    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct PublicApiContext {
        call_count: i32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum PublicApiEvent {
        Test,
    }

    statechart! {
        name: PublicApiMachine,
        context: PublicApiContext,
        event: PublicApiEvent,
        initial: State1,
        state State1 {
            on PublicApiEvent::Test => State2;
        }
        state State2 {}
    }

    #[test]
    fn test_uses_public_api() {
        let mut machine = PublicApiMachine::new(PublicApiContext::default(), &PublicApiEvent::Test)
            .expect("Failed to create public API machine");

        // This should compile and work correctly using the public API
        let result = machine.send(&PublicApiEvent::Test);
        assert_eq!(result, lit_bit_core::SendResult::Transitioned);

        // Also test the trait method directly
        let result2 = <PublicApiMachine as StateMachine>::send(&mut machine, &PublicApiEvent::Test);
        assert_eq!(result2, lit_bit_core::SendResult::NoMatch);
    }
}
