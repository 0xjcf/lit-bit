use lit_bit_macro::statechart;

statechart! {
    name: TestMachine,
    context: (),
    event: (),
    initial: P,
    state P [parallel] { // Error: Parallel state 'P' must have at least two child regions.
        state R1 {}
    }
}
fn main() {}
