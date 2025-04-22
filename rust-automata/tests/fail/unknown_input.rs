use rust_automata::*;

#[derive(Default)]
pub struct S1;
#[derive(Default)]
pub struct I1;

#[automaton(
    inputs(),
    states(S1),
    outputs(),
    initial_state(S1),
    transitions(
        (S1, I1) -> (S1)   // I1 not declared
    )
)]
pub struct BadInput;

fn main() {}