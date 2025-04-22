//! A simple state machine that models a lock.
//!
//! You can open the lock with a key or break it with a drill.
//! When you open or close the lock, it makes a click sound.
//!
//! Check out the [`Lock`] struct for the state machine diagram.
use rust_automata::*;

/// All the states of the lock.
pub mod states {
    #[derive(Default)]
    pub struct Open;
    #[derive(Default)]
    pub struct Closed;
    #[derive(Default)]
    pub struct Broken;
}

/// All the inputs of the lock.
pub mod inputs {
    #[derive(Default)]
    pub struct Key;
    #[derive(Default)]
    pub struct Drill;
}

/// All the outputs of the lock.
pub mod outputs {
    #[derive(Default)]
    pub struct Click;
    #[derive(Default)]
    pub struct Yell;
}

#[state_machine(
    inputs(inputs::Key, inputs::Drill),
    states(states::Open, states::Closed, states::Broken),
    outputs(outputs::Click),
    transitions(
        (states::Open,   inputs::Key)   -> (states::Closed, outputs::Click),
        (states::Closed, inputs::Key)   -> (states::Open,   outputs::Click),
        (states::Open,   inputs::Drill) -> (states::Broken),
        (states::Closed, inputs::Drill) -> (states::Broken)
    )
)]
pub struct Lock;

#[test]
fn locking_replay() {
    let mut lock = StateMachine::new(Lock, states::Open);
    assert!(lock.state().is_open());
    assert!(lock.can_consume::<inputs::Key>());

    let _sound: outputs::Click = lock.relay(inputs::Key);
    assert!(lock.state().is_closed());

    // We can ignore the output using a helper function.
    lock.consume(inputs::Key);
    assert!(lock.state().is_open());

    lock.consume(inputs::Drill);
    assert!(lock.state().is_broken());
    assert!(!lock.can_consume::<inputs::Key>());
    assert!(!lock.can_consume::<inputs::Drill>());
}
