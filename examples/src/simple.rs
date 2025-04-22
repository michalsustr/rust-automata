//! A simple state machine with two inputs, two outputs and three states.
use rust_automata::*;

#[state_machine(
    inputs(I1, I2),
    states(S1, S2, S3),
    outputs(O1, O2),
    transitions(
        (S1, I1) -> (S2, O1),
        (S2, I2) -> (S3, O2),
        (S3, I1) -> (S1, O1),
        (S3) -> (S2, O1),
    ),
    generate_structs(true),
    derive(Debug, PartialEq),
)]
pub struct Example;

#[test]
fn simple_example() {
    let mut m = StateMachine::new(Example, S1);
    assert!(m.state().is_s1());

    m.consume(I1);
    assert!(m.state().is_s2());

    let output: O2 = m.relay(I2);
    assert!(m.state().is_s3());
    assert_eq!(output, O2);

    let output: O1 = m.produce();
    assert!(m.state().is_s2());
    assert_eq!(output, O1);
}
