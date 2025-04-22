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
    initial_state(S1),
    transitions(
        (S1, I1) -> (S2) = handle   // refers to a method named `handle`
    )
)]
pub struct MissingHandler;

// ‑‑ no `impl MissingHandler { fn handle … }` provided on purpose ‑‑

fn main() {}