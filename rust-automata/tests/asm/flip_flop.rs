use rust_automata::*;
use std::process::ExitCode;

#[state_machine(
    states(Flip, Flop),
    transitions(
        (Flip) -> (Flop),
        (Flop) -> (Flip),
    ),
    generate_structs(true),
    derive(Debug, PartialEq),
)]
pub struct FlipFlop;


fn main() -> ExitCode {
    let mut m = StateMachine::new(FlipFlop, Flip);
    m.step();
    // If you remove one step call, the generated assembly will be still optimal.
    m.step();

    if m.state().is_flip() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
