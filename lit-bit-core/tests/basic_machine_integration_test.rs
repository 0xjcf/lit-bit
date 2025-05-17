// lit-bit-core/tests/basic_machine_integration_test.rs

// Items defined at the crate root of this integration test crate
// use lit_bit_core::core::{StateMachine, Runtime, StateNode, Transition, ActionFn, GuardFn}; // For generated code by macro
// The macro itself generates `use crate::core::...` so this top-level one might not be strictly needed for the generated code to compile,
// but could be useful if we directly used these types in the test file outside the macro.
// For now, let's rely on the macro's internal use statements.
// Let's keep it minimal for now.

// Items previously at crate root are now moved inside the test module.

#[cfg(test)]
mod basic_machine_integration_test {
    // Ensure StateMachine trait is in scope for calling its methods on TestMachine
    use lit_bit_core::StateMachine;
    use lit_bit_macro::statechart; // Keep for test assertions if needed.

    // 1. Define Context and Event types (now inside this module)
    #[derive(Debug, Default, Clone, PartialEq)]
    #[allow(clippy::struct_excessive_bools)] // Allow excessive bools for this test-specific context
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

    // 2. Define action and guard functions (now inside this module)
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

    // 3. Use the statechart! macro
    statechart! {
        name: TestMachine,
        context: TestContext, // Should resolve as it's in the same module
        event: TestEvent,   // Should resolve
        initial: State1,

        state State1 {
            entry: entry_s1; // Should resolve
            exit: exit_s1;   // Should resolve
            on Increment [guard guard_for_increment] => State2 [action transition_action_for_increment]; // Should resolve
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

        // Assuming non-parallel state, the Vec will contain one item.
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        assert!(
            machine.context().entry_action_called,
            "State1 entry action should have been called on init"
        );
        assert_eq!(machine.context().count, 0);
        assert!(!machine.context().exit_action_called);
        assert!(!machine.context().transition_action_called);

        machine.context_mut().entry_action_called = false;

        let transition_occurred_1 = machine.send(TestEvent::Increment);
        assert!(
            transition_occurred_1,
            "Expected a transition for first Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        assert!(machine.context().exit_action_called);
        assert!(machine.context().transition_action_called);
        assert_eq!(machine.context().count, 1);

        machine.context_mut().exit_action_called = false;
        machine.context_mut().transition_action_called = false;

        let transition_occurred_reset = machine.send(TestEvent::Reset);
        assert!(transition_occurred_reset, "Expected a transition for Reset");
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        assert!(machine.context().entry_action_called);
        machine.context_mut().entry_action_called = false;
        machine.context_mut().exit_action_called = false;

        let transition_occurred_inc2 = machine.send(TestEvent::Increment);
        assert!(
            transition_occurred_inc2,
            "Expected a transition for second Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State2]);
        assert_eq!(machine.context().count, 2);
        assert!(machine.context().exit_action_called);
        assert!(machine.context().transition_action_called);

        machine.context_mut().exit_action_called = false;
        machine.context_mut().transition_action_called = false;
        machine.context_mut().entry_action_called = false;

        // This send should trigger the State2 -> State1 transition (Reset)
        let transition_occurred_reset_2 = machine.send(TestEvent::Reset);
        assert!(
            transition_occurred_reset_2,
            "Expected a transition for second Reset"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        assert!(
            machine.context().entry_action_called,
            "Entry action for State1 should be called on Reset"
        );
        machine.context_mut().entry_action_called = false;
        machine.context_mut().exit_action_called = false;

        // This Increment should be blocked by the guard (count is 2, guard is count < 2)
        let transition_occurred_blocked = machine.send(TestEvent::Increment);
        assert!(
            !transition_occurred_blocked,
            "Expected no transition for blocked Increment"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        assert!(
            !machine.context().exit_action_called,
            "Exit action for State1 should not be called if transition is blocked"
        );
        assert!(
            !machine.context().transition_action_called,
            "Transition action should not be called if transition is blocked"
        );
        assert_eq!(
            machine.context().count,
            2,
            "Count should remain 2 if transition is blocked"
        ); // Count is incremented by action, so if no action, count remains.

        machine.context_mut().entry_action_called = false;
        machine.context_mut().exit_action_called = false;

        let transition_occurred_decrement = machine.send(TestEvent::Decrement);
        assert!(
            transition_occurred_decrement,
            "Expected a transition for Decrement"
        );
        assert_eq!(machine.state().as_slice(), &[TestMachineStateId::State1]);
        assert!(!machine.context().entry_action_called); // No entry for State1 if already in State1 and re-entering (unless specific re-entry action defined)
        assert!(!machine.context().exit_action_called); // No exit from State1 if target is State1
    }
}
