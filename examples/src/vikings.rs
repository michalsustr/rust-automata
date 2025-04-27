//! Example of the classic “four vikings and one torch” bridge‑crossing puzzle.
//!
//! Four vikings are about to cross a damaged bridge in the middle of the
//! night. The bridge can only carry two of the vikings at the time and to
//! find the way over the bridge the vikings need to bring a torch. The
//! vikings need 5, 10, 20 and 25 minutes (one-way) respectively to cross
//! the bridge.
//!
//! Does a schedule exist which gets all four vikings over the bridge
//! within 60 minutes?

use rust_automata::{clock::*, timestamp::*, *};

pub mod events {
    /// Viking tries to grab the torch
    #[derive(Default)]
    pub struct Take;
    /// Viking puts the torch down
    #[derive(Default)]
    pub struct Release;
}

pub mod viking_states {
    #[derive(Default)]
    pub struct UnsafeSide;
    #[derive(Default)]
    pub struct CrossingToSafe;
    #[derive(Debug, PartialEq, Default)]
    pub struct SafeSide;
    #[derive(Default)]
    pub struct CrossingToUnsafe;
}

#[state_machine(
    // Vikings *emit* Take/Release, they do not listen to anything external
    inputs(),
    states(
        viking_states::UnsafeSide,
        viking_states::CrossingToSafe,
        viking_states::SafeSide,
        viking_states::CrossingToUnsafe
    ),
    outputs(events::Take, events::Release),
    transitions(
        // Take torch on the unsafe side, start crossing → emit `Take`
        (viking_states::UnsafeSide)       -> (viking_states::CrossingToSafe, events::Take)   = reset_stopwatch,
        // Arrive on the safe side, put torch down      → emit `Release`
        (viking_states::CrossingToSafe)   -> (viking_states::SafeSide, events::Release)      : check_delay,

        // Take torch on the safe side, start back      → emit `Take`
        (viking_states::SafeSide)         -> (viking_states::CrossingToUnsafe, events::Take) = reset_stopwatch,
        // Arrive on the unsafe side, put torch down    → emit `Release`
        (viking_states::CrossingToUnsafe) -> (viking_states::UnsafeSide, events::Release)    : check_delay,
    )
)]
pub struct Viking {
    timer: Timer,
}

impl Viking {
    pub fn fsm(clock: &dyn Clock, delay: TimestampDelta) -> StateMachine<Self> {
        let timer = Timer::new(clock.clone_box(), delay);
        StateMachine::new(Self { timer }, viking_states::UnsafeSide)
    }

    fn reset_stopwatch(&mut self) {
        self.timer.reset();
    }

    fn check_delay(&self) -> bool {
        self.timer.is_timeout()
    }
}

pub mod torch_states {
    #[derive(Default)]
    pub struct Free;
    #[derive(Default)]
    pub struct One;
    #[derive(Default)]
    pub struct Two;
}

#[derive(Debug, PartialEq)]
enum TorchSide {
    Unsafe,
    Safe,
}

#[state_machine(
    inputs(events::Take, events::Release),  // listens to vikings
    states(torch_states::Free, torch_states::One, torch_states::Two),
    outputs(),  // torch produces no events
    transitions(
        // Someone grabs the torch
        (torch_states::Free, events::Take)    -> (torch_states::One),
        (torch_states::One, events::Take)     -> (torch_states::Two),
        // Someone puts the torch down
        (torch_states::Two,  events::Release) -> (torch_states::One),
        (torch_states::One, events::Release)  -> (torch_states::Free) = switch_side,
    )
)]
pub struct Torch {
    side: TorchSide,
}

impl Torch {
    pub fn fsm() -> StateMachine<Self> {
        StateMachine::new(
            Self {
                side: TorchSide::Unsafe,
            },
            torch_states::Free,
        )
    }

    fn switch_side(&mut self) {
        self.side = match self.side {
            TorchSide::Safe => TorchSide::Unsafe,
            TorchSide::Unsafe => TorchSide::Safe,
        }
    }
}

#[test]
fn vikings_successfully_cross() {
    use events::*;

    let clock = ManualClock::new();

    let mut v_fastest = Viking::fsm(&clock, TimestampDelta::from_minutes(5));
    let mut v_fast = Viking::fsm(&clock, TimestampDelta::from_minutes(10));
    let mut v_slow = Viking::fsm(&clock, TimestampDelta::from_minutes(20));
    let mut v_slowest = Viking::fsm(&clock, TimestampDelta::from_minutes(25));
    let mut torch = Torch::fsm();

    // One possible schedule (not necessarily optimal):

    // Step 1: V1 + V2 cross -> 10 min
    assert!(torch.can_consume::<Take>());
    assert!(v_fastest.can_produce::<Take>());
    assert!(v_fast.can_produce::<Take>());
    torch.consume(v_fastest.produce::<Take>());
    torch.consume(v_fast.produce::<Take>());
    assert!(!torch.can_consume::<Take>());
    assert!(!v_fastest.can_produce::<Take>());
    assert!(!v_fast.can_produce::<Take>());
    clock.advance_by(TimestampDelta::from_minutes(10));
    torch.consume(v_fastest.produce::<Release>());
    torch.consume(v_fast.produce::<Release>());

    // Step 2: V1 returns -> 5 min
    torch.consume(v_fastest.produce::<Take>());
    clock.advance_by(TimestampDelta::from_minutes(5));
    torch.consume(v_fastest.produce::<Release>());

    // Step 3: V3 + V4 cross -> 25 min
    torch.consume(v_slow.produce::<Take>());
    torch.consume(v_slowest.produce::<Take>());
    clock.advance_by(TimestampDelta::from_minutes(25));
    torch.consume(v_slow.produce::<Release>());
    torch.consume(v_slowest.produce::<Release>());

    // Step 4: V2 returns -> 10 min
    torch.consume(v_fast.produce::<Take>());
    clock.advance_by(TimestampDelta::from_minutes(10));
    torch.consume(v_fast.produce::<Release>());

    // Step 5: V1 + V2 cross -> 10 min
    torch.consume(v_fastest.produce::<Take>());
    torch.consume(v_fast.produce::<Take>());
    clock.advance_by(TimestampDelta::from_minutes(10));
    torch.consume(v_fastest.produce::<Release>());
    torch.consume(v_fast.produce::<Release>());

    // All ok.
    assert!(!torch.state().is_failure());
    assert!(!v_fastest.state().is_failure());
    assert!(!v_fast.state().is_failure());
    assert!(!v_slow.state().is_failure());
    assert!(!v_slowest.state().is_failure());
    // Total time: 60 minutes
    assert_eq!(clock.now(), Timestamp::from_minutes(60));
    // Check that the vikings are on the safe side.
    assert!(v_fastest.state().is_safe_side());
    assert!(v_fast.state().is_safe_side());
    assert!(v_slow.state().is_safe_side());
    assert!(v_slowest.state().is_safe_side());
    // Check that the torch is free.
    assert!(torch.state().is_free());
    assert_eq!(torch.data().side, TorchSide::Safe);
}
