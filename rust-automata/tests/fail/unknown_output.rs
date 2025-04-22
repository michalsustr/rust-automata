use rust_automata::*;

#[derive(Default)]
pub struct S1;
#[derive(Default)]
pub struct I1;
#[derive(Default)]
pub struct O1;

#[automaton(
    inputs(I1),
    states(S1),
    outputs(),           // O1 NOT listed
    initial_state(S1),
    transitions(
        (S1, I1) -> (S1, O1)
    )
)]
pub struct BadOutput;

fn main() {}