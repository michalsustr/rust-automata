//! Attribute‑style DSL for defining finite‑state machines.
//!
//! See the [rust-automata crate](https://crates.io/crates/rust-automata/) for high-level usage.

pub use rust_automata_macros::state_machine;
pub use rust_automata_macros::Display;

#[doc(hidden)]
#[cfg(feature = "mermaid")]
pub use aquamarine::aquamarine;

pub mod clock;
#[doc(hidden)]
mod takeable;
pub mod timestamp;

use core::fmt::Display;
use std::hash::Hash;
use std::marker::PhantomData;
use log;

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
    fn get_variant(id: &EnumId<ForEnum>) -> &'static str;
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
    /// The input alphabet enum.
    type Input: Alphabet + Enumerable<Self::Input>;
    /// The possible states enum.
    type State: StateTrait + Enumerable<Self::State>;
    /// The output alphabet enum.
    type Output: Alphabet + Enumerable<Self::Output>;
    /// The initial state (an actual enum value). May be needed to be supplied manually by the user.
    type InitialState: Enumerated<Self::State> + Into<Self::State>;
    /// The nothing input/output symbol.
    type Nothing: Enumerated<Self::Input> + Enumerated<Self::Output> + Into<Self::Input> + From<Self::Output> + Default;
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
    /// The name of the state machine.
    fn name() -> &'static str;
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
    #[inline]
    pub fn step(&mut self) {
        self.relay::<T::Nothing, T::Nothing>(T::Nothing::default());
    }

    /// Produce an output, given no input.
    #[inline]
    pub fn produce<O: From<T::Output> + Enumerated<T::Output>>(&mut self) -> O {
        self.relay::<T::Nothing, O>(T::Nothing::default())
    }

    /// Consume an input, do not care about the output.
    #[inline]
    pub fn consume<I: Into<T::Input> + Enumerated<T::Input>>(&mut self, input: I) {
        self.relay::<I, T::Output>(input);
    }

    /// Consume an input, produce an output.
    #[inline]
    pub fn relay<I: Into<T::Input> + Enumerated<T::Input>, O: From<T::Output>>(&mut self, input: I) -> O {
        let enum_input: T::Input = input.into();
        // Store only the ids so we don't have to prematurely call `to_string` on the enums.
        let from_id = self.state.as_ref().enum_id();
        let input_id = enum_input.enum_id();

        // Take ownership of the current state
        let current_state = std::mem::replace(&mut self.state, Takeable::new(T::State::failure()));

        // Call transition with owned state
        let (next_state, output) = self.data.transition(current_state, enum_input);

        // Update state with the result
        self.state = next_state;

        if self.state.is_failure() {
            let from_str = T::State::get_variant(&from_id);
            let input_str = T::Input::get_variant(&input_id);
            panic!("Invalid transition from {from_str} using input {input_str}");
        } else {
            log::debug!("{}: ({}, {}) -> ({}, {})", 
                T::name(),
                T::State::get_variant(&from_id), 
                T::Input::get_variant(&input_id),
                T::State::get_variant(&self.state.as_ref().enum_id()),
                T::Output::get_variant(&output.enum_id()),
            );
        }
        O::from(output)
    }

    #[inline]
    pub fn can_step(&mut self) -> bool {
        let enum_input = T::Nothing::enum_id();
        let enum_state = self.state.as_ref();
        let actual_output = self.data.can_transition(enum_state, enum_input);
        actual_output.is_some()
    }

    #[inline]
    pub fn can_produce<O>(&mut self) -> bool
    where
        O: Enumerated<T::Output>,
    {
        let enum_input = T::Nothing::enum_id();
        let enum_state = self.state.as_ref();
        let actual_output = self.data.can_transition(enum_state, enum_input);
        let expected_enum = O::enum_id();
        match actual_output {
            Some(enum_output) => enum_output == expected_enum,
            None => false,
        }
    }

    #[inline]
    pub fn can_consume<I>(&mut self) -> bool
    where
        I: Enumerated<T::Input>,
    {
        let enum_input = I::enum_id();
        let enum_state = self.state.as_ref();
        let actual_output = self.data.can_transition(enum_state, enum_input);
        actual_output.is_some()
    }

    #[inline]
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
