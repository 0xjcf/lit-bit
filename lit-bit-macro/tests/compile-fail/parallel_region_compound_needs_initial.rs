use lit_bit_macro::statechart;

statechart! {
    name: TestMachine,
    context: (),
    event: (),
    initial: P,
    state P [parallel] {
        state R1 { // Error: Region 'P_R1' within parallel state 'P' is a compound state and must declare an 'initial' child.
            state S1 {}
        }
        state R2 {}
    }
}
fn main() {}
