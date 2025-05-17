use lit_bit_macro::statechart;

statechart! {
    name: TestMachine,
    context: (),
    event: (),
    initial: P,
    state P [parallel] {
        initial: R1; // Error: Parallel state 'P' must not declare an 'initial' child for itself.
        state R1 {}
        state R2 {}
    }
}
fn main() {}
