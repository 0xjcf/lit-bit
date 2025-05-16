use lit_bit_macro::statechart;

// Define a context and event type for the test macro
#[derive(Clone, Debug, Default)]
struct TestContext;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum TestEvent {
    Go,
}

statechart! {
    name: MyMachine,
    context: TestContext,
    event: TestEvent,
    initial: StateA,

    state StateA {
        on Go => NonExistentState; // Error: NonExistentState is not defined
    }
    // StateB is defined but not NonExistentState
    state StateB {}
}

fn main() {}
