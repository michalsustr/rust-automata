use rust_automata::*;

#[derive(Default)]
pub struct S1;
#[derive(Default)]
pub struct I1;

#[automaton(
    inputs(I1),
    states(S1),
    outputs(),
    initial_state(S1),
    transitions(
        (S1, I1) -> (S2)   // S2 is not declared
    )
)]
pub struct BadState;

fn main() {}
