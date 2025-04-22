use rust_automata::*;

// States
#[derive(Default)]
pub struct S1;
#[derive(Default)]
pub struct S2;

// Input
#[derive(Default)]
pub struct I1;

#[automaton(
    inputs(I1),
    states(S1, S2),
    outputs(),
    transitions(
        (S1, I1) -> (S2) = handle_transition
    )
)]
pub struct HandlerTypeMismatch;

impl HandlerTypeMismatch {
    // Should return S2
    fn handle_transition(&mut self, _: S1, _: I1) -> bool { 
        true
    }
}

fn main() {}