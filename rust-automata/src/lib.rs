//! Attribute‑style DSL for defining finite‑state machines.
//!
//! See the [rust-automata crate](https://crates.io/crates/rust-automata/) for high-level usage.

pub use rust_automata_macros::state_machine;
pub use rust_automata_macros::Display;

#[doc(hidden)]
#[cfg(feature = "mermaid")]
pub use aquamarine::aquamarine;

pub mod clock;
mod takeable;
pub mod timestamp;

use core::fmt::Display;
use std::hash::Hash;
use std::marker::PhantomData;

#[doc(hidden)]
pub use takeable::Takeable;

/// Trait for input/output alphabet. Used for internal enum generation.
///
/// All the input structs are enumerated in an internal enum that implements this trait.
pub trait Alphabet: Display {
    /// Return a value that represents no input/output.
    fn nothing() -> Self;
    /// Check if the alphabet is anything other than `nothing`.
    fn any(&self) -> bool;
}

/// Trait for states. Used for internal enum generation.
///
/// All the state structs are enumerated in an internal enum that implements this trait.
pub trait StateTrait: Display {
    fn failure() -> Self;
    fn is_failure(&self) -> bool;
}

// Get id in the enum wrapper. For internal use only.
#[doc(hidden)]
pub trait Enumerable<ForEnum> {
    fn enum_id(&self) -> EnumId<ForEnum>;
}

// Get id in the wrapped struct. For internal use only.
#[doc(hidden)]
pub trait Enumerated<InEnum> {
    fn enum_id() -> EnumId<InEnum>;
}

/// For internal use only.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialOrd, Ord, Default)]
pub struct EnumId<ForEnum> {
    pub id: usize,
    _marker: PhantomData<ForEnum>,
}

impl<ForEnum> PartialEq for EnumId<ForEnum> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<ForEnum> Hash for EnumId<ForEnum> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<ForEnum> EnumId<ForEnum> {
    pub fn new(id: usize) -> Self {
        EnumId {
            id,
            _marker: PhantomData,
        }
    }
}

/// Describe any possible deterministic finite state  machine/transducer.
///
/// This is just a formal definition that may be inconvenient to be used in practical programming,
/// but it is used throughout this library for more practical things.
///
/// For internal use only.
#[doc(hidden)]
pub trait StateMachineImpl {
    /// The input alphabet.
    type Input: Alphabet;
    /// The set of possible states.
    type State: StateTrait + Enumerable<Self::State>;
    /// The output alphabet.
    type Output: Alphabet;
    /// The initial state. May be needed to be supplied manually by the user.
    type InitialState: Enumerated<Self::State> + Into<Self::State>;
    /// The transition function that takes ownership of the current state and returns
    /// a new state along with any output based on the provided input.
    fn transition(
        &mut self,
        state: Takeable<Self::State>,
        input: Self::Input,
    ) -> (Takeable<Self::State>, Self::Output);
    /// Check if a transition is possible. If yes, return the output enum id.
    fn can_transition(
        &self,
        state: &Self::State,
        input: EnumId<Self::Input>,
    ) -> Option<EnumId<Self::Output>>;
}

/// Encapsulates the state and other SM data and expose transition functions.
pub struct StateMachine<T: StateMachineImpl> {
    state: Takeable<T::State>,
    data: T,
}

impl<T> StateMachine<T>
where
    T: StateMachineImpl,
{
    /// Create a new instance of this wrapper which encapsulates the initial state.
    pub fn new(data: T, initial_state: T::InitialState) -> Self {
        Self {
            state: Takeable::new(initial_state.into()),
            data,
        }
    }

    /// Only change the state, do not accept any input and do not produce any output.
    pub fn step(&mut self) {
        let _: T::Output = self.relay::<T::Input, T::Output>(T::Input::nothing());
    }

    /// Produce an output, given no input.
    pub fn produce<O: From<T::Output>>(&mut self) -> O {
        self.relay::<T::Input, O>(T::Input::nothing())
    }

    /// Consume an input, do not care about the output.
    pub fn consume<I: Into<T::Input>>(&mut self, input: I) {
        let _: T::Output = self.relay::<I, T::Output>(input);
    }

    /// Consume an input, produce an output.
    pub fn relay<I: Into<T::Input>, O: From<T::Output>>(&mut self, input: I) -> O {
        let enum_input = input.into();
        let from_str = self.state.as_ref().to_string();
        let input_str = enum_input.to_string();

        // Take ownership of the current state
        let current_state = std::mem::replace(&mut self.state, Takeable::new(T::State::failure()));

        // Call transition with owned state
        let (next_state, output) = self.data.transition(current_state, enum_input);

        // Update state with the result
        self.state = next_state;

        if self.state.is_failure() {
            panic!(
                "Invalid transition from {} using input {}",
                from_str, input_str
            );
        }

        O::from(output)
    }

    pub fn can_step(&mut self) -> bool {
        let enum_input = EnumId::new(0);
        let enum_state = self.state.as_ref();
        let actual_output = self.data.can_transition(enum_state, enum_input);
        actual_output.is_some()
    }

    pub fn can_produce<O>(&mut self) -> bool
    where
        O: Enumerated<T::Output>,
    {
        let enum_input = EnumId::new(0);
        let enum_state = self.state.as_ref();
        let actual_output = self.data.can_transition(enum_state, enum_input);
        let expected_enum = O::enum_id();
        match actual_output {
            Some(enum_output) => enum_output == expected_enum,
            None => false,
        }
    }

    pub fn can_consume<I>(&mut self) -> bool
    where
        I: Enumerated<T::Input>,
    {
        let enum_input = I::enum_id();
        let enum_state = self.state.as_ref();
        let actual_output = self.data.can_transition(enum_state, enum_input);
        actual_output.is_some()
    }

    pub fn can_relay<I, O>(&mut self) -> bool
    where
        I: Enumerated<T::Input>,
        O: Enumerated<T::Output>,
    {
        let enum_input = I::enum_id();
        let enum_state = self.state.as_ref();
        let actual_output = self.data.can_transition(enum_state, enum_input);
        let expected_enum = O::enum_id();
        match actual_output {
            Some(enum_output) => enum_output == expected_enum,
            None => false,
        }
    }

    /// Returns the current state.
    pub fn state(&self) -> &T::State {
        &self.state
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}
