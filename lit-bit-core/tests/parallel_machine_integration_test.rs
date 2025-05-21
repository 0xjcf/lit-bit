// lit-bit-core/tests/parallel_machine_integration_test.rs

#[cfg(test)]
mod parallel_integration_tests {
    use core::convert::TryFrom; // For heapless::String::try_from
    use heapless::String;
    use lit_bit_core::StateMachine; // Trait needed for machine.state(), machine.send(), etc.
    use lit_bit_macro::statechart; // The proc macro // For string manipulation in context/log

    // Define a context for logging actions, similar to unit tests
    const LOG_CAPACITY: usize = 32;
    const STRING_CAPACITY: usize = 64;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct TestLogContext {
        log: heapless::Vec<String<STRING_CAPACITY>, LOG_CAPACITY>,
    }

    impl TestLogContext {
        fn record(&mut self, action_name: &str) {
            self.log
                .push(String::try_from(action_name).unwrap())
                .expect("Log overflow");
        }
        #[allow(dead_code)] // Will be used in assertions
        fn get_log_str_slice(&self) -> heapless::Vec<&str, LOG_CAPACITY> {
            self.log.iter().map(heapless::String::as_str).collect()
        }
    }

    // Helper function for checking subsequences in logs for this integration test module
    fn check_subsequence_integration(
        log: &heapless::Vec<&str, LOG_CAPACITY>,
        expected_sub: &[&str],
    ) -> bool {
        if expected_sub.is_empty() {
            return true;
        }
        let mut log_idx = 0;
        let mut sub_idx = 0;
        while log_idx < log.len() && sub_idx < expected_sub.len() {
            if log[log_idx] == expected_sub[sub_idx] {
                sub_idx += 1;
            }
            log_idx += 1;
        }
        sub_idx == expected_sub.len()
    }

    // Define events
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum TestEvent {
        EvGlobal,
        EvR1,
        EvR2,
        EvPSelf,
        EvToOuter,
        EvToP,
    }

    // Action logging functions
    fn log_enter_parallel(ctx: &mut TestLogContext) {
        ctx.record("EnterParallel");
    }
    fn log_exit_parallel(ctx: &mut TestLogContext) {
        ctx.record("ExitParallel");
    }
    fn log_action_p_self(ctx: &mut TestLogContext) {
        ctx.record("ActionPSelf");
    }
    fn log_action_to_outer(ctx: &mut TestLogContext) {
        ctx.record("ActionToOuter");
    }
    fn log_action_to_p(ctx: &mut TestLogContext) {
        ctx.record("ActionToP");
    }

    fn log_enter_r1(ctx: &mut TestLogContext) {
        ctx.record("EnterR1");
    }
    fn log_exit_r1(ctx: &mut TestLogContext) {
        ctx.record("ExitR1");
    }
    fn log_enter_r1a(ctx: &mut TestLogContext) {
        ctx.record("EnterR1A");
    }
    fn log_exit_r1a(ctx: &mut TestLogContext) {
        ctx.record("ExitR1A");
    }
    fn log_action_r1_ev_r1(ctx: &mut TestLogContext) {
        ctx.record("ActionR1EvR1");
    }

    fn log_enter_r2(ctx: &mut TestLogContext) {
        ctx.record("EnterR2");
    }
    fn log_exit_r2(ctx: &mut TestLogContext) {
        ctx.record("ExitR2");
    }
    fn log_enter_r2x(ctx: &mut TestLogContext) {
        ctx.record("EnterR2X");
    }
    fn log_exit_r2x(ctx: &mut TestLogContext) {
        ctx.record("ExitR2X");
    }
    fn log_action_r2_ev_r2(ctx: &mut TestLogContext) {
        ctx.record("ActionR2EvR2");
    }

    fn log_enter_outer(ctx: &mut TestLogContext) {
        ctx.record("EnterOuter");
    }
    fn log_exit_outer(ctx: &mut TestLogContext) {
        ctx.record("ExitOuter");
    }

    // Define the statechart using the macro
    statechart! {
        name: IntegrationParallelMachine,
        context: TestLogContext,
        event: TestEvent,
        initial: ParallelState, // Start in the parallel state

        state ParallelState [parallel] {
            entry: log_enter_parallel;
            exit: log_exit_parallel;
            on EvPSelf => ParallelState [action log_action_p_self];
            on EvToOuter => OuterState [action log_action_to_outer];

            state Region1 {
                initial: Region1StateA;
                entry: log_enter_r1;
                exit: log_exit_r1;
                state Region1StateA {
                    entry: log_enter_r1a;
                    exit: log_exit_r1a;
                    on EvR1 => Region1StateA [action log_action_r1_ev_r1];
                    on EvGlobal => Region1StateA;
                }
            }

            state Region2 {
                initial: Region2StateX;
                entry: log_enter_r2;
                exit: log_exit_r2;
                state Region2StateX {
                    entry: log_enter_r2x;
                    exit: log_exit_r2x;
                    on EvR2 => Region2StateX [action log_action_r2_ev_r2];
                    on EvGlobal => Region2StateX;
                }
            }
        }

        state OuterState {
            entry: log_enter_outer;
            exit: log_exit_outer;
            on EvToP => ParallelState [action log_action_to_p];
        }
    }

    #[test]
    fn test_parallel_machine_initialization() {
        let machine = IntegrationParallelMachine::new(TestLogContext::default());
        let active_states = machine.state();

        let mut active_state_strings = active_states
            .iter()
            .map(|s| {
                heapless::String::<STRING_CAPACITY>::try_from(format!("{s:?}").as_str()).unwrap()
            })
            .collect::<heapless::Vec<
                heapless::String<STRING_CAPACITY>,
                { lit_bit_core::core::MAX_ACTIVE_REGIONS },
            >>();
        active_state_strings.sort_unstable();

        let mut expected_state_strings = heapless::Vec::<
            heapless::String<STRING_CAPACITY>,
            { lit_bit_core::core::MAX_ACTIVE_REGIONS },
        >::new();
        // Match Debug output of StateId enum variants (no underscores)
        expected_state_strings
            .push(String::try_from("ParallelStateRegion1Region1StateA").unwrap())
            .unwrap();
        expected_state_strings
            .push(String::try_from("ParallelStateRegion2Region2StateX").unwrap())
            .unwrap();
        expected_state_strings.sort_unstable();

        assert_eq!(
            active_state_strings, expected_state_strings,
            "Initial active states mismatch"
        );

        let expected_log_entries = [
            "EnterParallel",
            "EnterR1",
            "EnterR1A",
            "EnterR2",
            "EnterR2X",
        ];
        let actual_log_strs = machine.context().get_log_str_slice();
        assert_eq!(
            actual_log_strs.as_slice(),
            &expected_log_entries,
            "Initial entry log mismatch"
        );
    }

    #[test]
    fn test_integration_parallel_independent_region_events() {
        let mut machine = IntegrationParallelMachine::new(TestLogContext::default());
        machine.context_mut().log.clear();

        let transitioned = machine.send(TestEvent::EvGlobal);
        assert!(transitioned, "Expected EvGlobal to cause transitions");

        let active_states = machine.state();
        let mut active_state_strings = active_states
            .iter()
            .map(|s| {
                heapless::String::<STRING_CAPACITY>::try_from(format!("{s:?}").as_str()).unwrap()
            })
            .collect::<heapless::Vec<
                heapless::String<STRING_CAPACITY>,
                { lit_bit_core::core::MAX_ACTIVE_REGIONS },
            >>();
        active_state_strings.sort_unstable();

        let mut expected_state_strings = heapless::Vec::<
            heapless::String<STRING_CAPACITY>,
            { lit_bit_core::core::MAX_ACTIVE_REGIONS },
        >::new();
        expected_state_strings
            .push(String::try_from("ParallelStateRegion1Region1StateA").unwrap())
            .unwrap();
        expected_state_strings
            .push(String::try_from("ParallelStateRegion2Region2StateX").unwrap())
            .unwrap();
        expected_state_strings.sort_unstable();
        assert_eq!(
            active_state_strings, expected_state_strings,
            "Active states should remain after EvGlobal self-transitions"
        );

        let actual_log_strs = machine.context().get_log_str_slice();
        let r1_actions_expected_slice = &["ExitR1A", "EnterR1A"];
        let r2_actions_expected_slice = &["ExitR2X", "EnterR2X"];

        assert!(
            check_subsequence_integration(&actual_log_strs, r1_actions_expected_slice),
            "R1A self-transition log {r1_actions_expected_slice:?} not found. Log: {actual_log_strs:?}"
        );
        assert!(
            check_subsequence_integration(&actual_log_strs, r2_actions_expected_slice),
            "R2X self-transition log {r2_actions_expected_slice:?} not found. Log: {actual_log_strs:?}"
        );

        assert_eq!(
            actual_log_strs.len(),
            r1_actions_expected_slice.len() + r2_actions_expected_slice.len(),
            "Log length mismatch. Expected {}, got {}. Log: {actual_log_strs:?}",
            r1_actions_expected_slice.len() + r2_actions_expected_slice.len(),
            actual_log_strs.len()
        );
    }

    #[test]
    fn test_integration_parallel_self_transition_on_parallel_state() {
        let mut machine = IntegrationParallelMachine::new(TestLogContext::default());
        // Initial state: ParallelState with R1A and R2X active.
        // Initial log: EnterParallel, EnterR1, EnterR1A, EnterR2, EnterR2X
        machine.context_mut().log.clear();

        let transitioned = machine.send(TestEvent::EvPSelf);
        assert!(transitioned, "Expected EvPSelf to cause a transition");

        // Active states should remain the same initial leaf states of the regions
        let active_states = machine.state();
        let mut active_state_strings = active_states
            .iter()
            .map(|s| {
                heapless::String::<STRING_CAPACITY>::try_from(format!("{s:?}").as_str()).unwrap()
            })
            .collect::<heapless::Vec<
                heapless::String<STRING_CAPACITY>,
                { lit_bit_core::core::MAX_ACTIVE_REGIONS },
            >>();
        active_state_strings.sort_unstable();

        let mut expected_state_strings = heapless::Vec::<
            heapless::String<STRING_CAPACITY>,
            { lit_bit_core::core::MAX_ACTIVE_REGIONS },
        >::new();
        expected_state_strings
            .push(String::try_from("ParallelStateRegion1Region1StateA").unwrap())
            .unwrap();
        expected_state_strings
            .push(String::try_from("ParallelStateRegion2Region2StateX").unwrap())
            .unwrap();
        expected_state_strings.sort_unstable();
        assert_eq!(
            active_state_strings, expected_state_strings,
            "Active states should be reset to initial regional states after ParallelState self-transition"
        );

        let expected_log_sequence = [
            // Order of region exits might depend on definition, but both must exit before ParallelState
            "ExitR1A",
            "ExitR1",
            "ExitR2X",
            "ExitR2",
            "ExitParallel",
            "ActionPSelf",
            "EnterParallel",
            // Order of region entries might depend on definition, but both must enter after ParallelState
            "EnterR1",
            "EnterR1A",
            "EnterR2",
            "EnterR2X",
        ];
        let actual_log_strs = machine.context().get_log_str_slice();
        // For this specific complex sequence, an exact match is important to verify full re-entry.
        assert_eq!(
            actual_log_strs.as_slice(),
            &expected_log_sequence,
            "Log for EvPSelf (ParallelState self-transition) mismatch"
        );
    }

    #[test]
    fn test_integration_transition_from_parallel_to_outer() {
        let mut machine = IntegrationParallelMachine::new(TestLogContext::default());
        machine.context_mut().log.clear();

        let transitioned = machine.send(TestEvent::EvToOuter);
        assert!(transitioned, "Expected EvToOuter to cause a transition");

        let active_states = machine.state();
        let mut active_state_strings = active_states
            .iter()
            .map(|s| {
                heapless::String::<STRING_CAPACITY>::try_from(format!("{s:?}").as_str()).unwrap()
            })
            .collect::<heapless::Vec<
                heapless::String<STRING_CAPACITY>,
                { lit_bit_core::core::MAX_ACTIVE_REGIONS },
            >>();
        // Only one active state expected, no sort needed but done for consistency if future changes add more.
        active_state_strings.sort_unstable();

        let mut expected_state_strings = heapless::Vec::<
            heapless::String<STRING_CAPACITY>,
            { lit_bit_core::core::MAX_ACTIVE_REGIONS },
        >::new();
        expected_state_strings
            .push(String::try_from("OuterState").unwrap())
            .unwrap();
        expected_state_strings.sort_unstable();
        assert_eq!(
            active_state_strings, expected_state_strings,
            "Active state should be OuterState"
        );

        let expected_log_sequence = [
            "ExitR1A",
            "ExitR1",
            "ExitR2X",
            "ExitR2",
            "ExitParallel",
            "ActionToOuter",
            "EnterOuter",
        ];
        let actual_log_strs = machine.context().get_log_str_slice();
        assert_eq!(
            actual_log_strs.as_slice(),
            &expected_log_sequence,
            "Log for EvToOuter (Parallel to Outer) mismatch"
        );
    }

    #[test]
    fn test_integration_transition_from_outer_to_parallel() {
        let mut machine = IntegrationParallelMachine::new(TestLogContext::default());
        // Initial state is ParallelState. Transition to OuterState first.
        let _ = machine.send(TestEvent::EvToOuter); // Log: ExitR1A, ExitR1, ExitR2X, ExitR2, ExitParallel, ActionToOuter, EnterOuter

        // Verify current state is OuterState before clearing log
        let current_intermediate_states = machine.state();
        assert_eq!(
            current_intermediate_states.len(),
            1,
            "Should be in OuterState before testing EvToP"
        );
        let current_intermediate_state_name = heapless::String::<STRING_CAPACITY>::try_from(
            format!("{:?}", current_intermediate_states[0]).as_str(),
        )
        .unwrap();
        assert_eq!(
            current_intermediate_state_name, "OuterState",
            "Machine not in OuterState before EvToP test"
        );

        machine.context_mut().log.clear(); // Clear log after setup

        let transitioned = machine.send(TestEvent::EvToP);
        assert!(
            transitioned,
            "Expected EvToP to cause a transition from OuterState to ParallelState"
        );

        let active_states = machine.state();
        let mut active_state_strings = active_states
            .iter()
            .map(|s| {
                heapless::String::<STRING_CAPACITY>::try_from(format!("{s:?}").as_str()).unwrap()
            })
            .collect::<heapless::Vec<
                heapless::String<STRING_CAPACITY>,
                { lit_bit_core::core::MAX_ACTIVE_REGIONS },
            >>();
        active_state_strings.sort_unstable();

        let mut expected_state_strings = heapless::Vec::<
            heapless::String<STRING_CAPACITY>,
            { lit_bit_core::core::MAX_ACTIVE_REGIONS },
        >::new();
        expected_state_strings
            .push(String::try_from("ParallelStateRegion1Region1StateA").unwrap())
            .unwrap();
        expected_state_strings
            .push(String::try_from("ParallelStateRegion2Region2StateX").unwrap())
            .unwrap();
        expected_state_strings.sort_unstable();
        assert_eq!(
            active_state_strings, expected_state_strings,
            "Active states should be regional children of ParallelState"
        );

        let expected_log_sequence = [
            "ExitOuter",
            "ActionToP",
            "EnterParallel",
            "EnterR1",
            "EnterR1A",
            "EnterR2",
            "EnterR2X",
        ];
        let actual_log_strs = machine.context().get_log_str_slice();
        assert_eq!(
            actual_log_strs.as_slice(),
            &expected_log_sequence,
            "Log for EvToP (Outer to Parallel) mismatch"
        );
    }

    #[test]
    fn test_integration_event_targets_specific_region() {
        let mut machine = IntegrationParallelMachine::new(TestLogContext::default());
        // Initial state: ParallelState with R1A and R2X active.
        machine.context_mut().log.clear();

        let transitioned = machine.send(TestEvent::EvR1);
        assert!(
            transitioned,
            "Expected EvR1 to cause a transition in Region1"
        );

        // Active states should remain the same as R1A has a self-transition and R2X is unaffected.
        let active_states = machine.state();
        let mut active_state_strings = active_states
            .iter()
            .map(|s| {
                heapless::String::<STRING_CAPACITY>::try_from(format!("{s:?}").as_str()).unwrap()
            })
            .collect::<heapless::Vec<
                heapless::String<STRING_CAPACITY>,
                { lit_bit_core::core::MAX_ACTIVE_REGIONS },
            >>();
        active_state_strings.sort_unstable();

        let mut expected_state_strings = heapless::Vec::<
            heapless::String<STRING_CAPACITY>,
            { lit_bit_core::core::MAX_ACTIVE_REGIONS },
        >::new();
        expected_state_strings
            .push(String::try_from("ParallelStateRegion1Region1StateA").unwrap())
            .unwrap();
        expected_state_strings
            .push(String::try_from("ParallelStateRegion2Region2StateX").unwrap())
            .unwrap();
        expected_state_strings.sort_unstable();
        assert_eq!(
            active_state_strings, expected_state_strings,
            "Active states mismatch after EvR1"
        );

        let expected_log_sequence = ["ExitR1A", "ActionR1EvR1", "EnterR1A"];
        let actual_log_strs = machine.context().get_log_str_slice();
        assert_eq!(
            actual_log_strs.as_slice(),
            &expected_log_sequence,
            "Log for EvR1 (target Region1) mismatch"
        );
    }

    // Further tests for parallel event handling will be added here.
}
