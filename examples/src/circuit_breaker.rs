//! Circuit Breaker pattern implementation.
//!
//! The *circuit breaker pattern* is a design pattern for building resilient systems
//! by preventing repeated failure requests to a service or operation that is likely to fail.
//! It improves system reliability, reduces strain on failing components, and helps ensure
//! that the system remains responsive even when part of it is in a degraded state.
//!
//! ### Motivation
//!
//! In distributed systems, it's common for one part of the system to fail, causing ripple
//! effects that result in even more failures (often referred to as *cascading failures*).
//! When one service is overloaded or malfunctioning, continuous attempts to access it can
//! make things worse by further overwhelming the service and other dependent services.
//!
//! The Circuit Breaker pattern helps mitigate this issue by "tripping" a breaker when a
//! service starts failing frequently, stopping further calls to the service until it has
//! had time to recover. This helps prevent unnecessary load on already failing components
//! and gives them a chance to recover without being constantly bombarded with requests.
//!
//! ### Automatic Recovery After Timeout
//!
//! One of the features of the circuit breaker pattern is *automatic recovery*. After
//! the breaker trips and enters the [`states::Open`] state due to a series of failures, it will stay
//! in that state for a pre-configured timeout period. Once the timeout elapses, the circuit
//! breaker will automatically transition to the [`states::HalfOpen`] state.
//!
//! In the [`states::HalfOpen`] state, the system attempts to make a small number of requests to see
//! if the underlying service has recovered. If the service responds successfully, the breaker
//! will transition back to the [`states::Closed`] state, and normal operations resume. If failures persist,
//! the breaker will revert to the [`states::Open`] state, and the recovery process starts again after
//! another timeout.
//!
//! This mechanism allows the system to "self-heal" without manual intervention, automatically
//! testing the service's availability after a failure period, and recovering once the service
//! is deemed stable again.
//!
//! ### Example Usage
//!
//! See [`FaultyRoute`] for an example of integrating the circuit breaker
//! in a web service scenario. This example demonstrates how the circuit breaker
//! can be used to prevent repeatedly calling a service that is in a failing state.
//!
//! ### Further Reading
//!
//! Based on [Martin Fowler’s blog post on Circuit Breakers](https://martinfowler.com/bliki/CircuitBreaker.html).

use rust_automata::clock::*;
use rust_automata::timestamp::TimestampDelta;
use rust_automata::*;

/// States of the circuit breaker.
pub mod states {
    /// The system is operating normally. Failures are counted.
    #[derive(PartialEq, Debug, Default)]
    pub struct Closed {
        pub count: u32,
    }
    /// The failure threshold has been exceeded. Requests are rejected.
    #[derive(Debug)]
    pub struct Open {
        pub timer: super::Timer,
    }
    /// A trial state to test if the system has recovered.
    #[derive(PartialEq, Debug, Default)]
    pub struct HalfOpen;
}

/// Inputs of the circuit breaker.
pub mod inputs {
    /// A successful request.
    #[derive(PartialEq, Debug, Default)]
    pub struct Success;
    /// A failed request.
    #[derive(PartialEq, Debug, Default)]
    pub struct Fail;
}

#[state_machine(
    inputs(inputs::Success, inputs::Fail),
    states(states::Closed, states::Open, states::HalfOpen),
    outputs(),
    transitions(
        (states::Closed, inputs::Success) -> (states::Closed) = handle_count_reset,
        (states::Closed, inputs::Fail)    -> (states::Closed) :  guard_below_threshold = handle_count_increment,
        (states::Closed, inputs::Fail)    -> (states::Open)   :  ! guard_below_threshold = handle_trip_breaker,

        (states::Open) -> (states::Open)     :  !guard_timeout,
        (states::Open) -> (states::HalfOpen) :  guard_timeout,

        (states::HalfOpen, inputs::Fail)    -> (states::Open) = handle_setup_timer,
        (states::HalfOpen, inputs::Success) -> (states::Closed)  // Resumes normal operation
    ),
    derive(Debug)
)]
pub struct CircuitBreaker {
    pub clock: Box<dyn Clock>,
    pub threshold: u32,
    pub timeout: TimestampDelta,
}

impl CircuitBreaker {
    fn guard_below_threshold(&self, closed: &states::Closed) -> bool {
        closed.count < self.threshold
    }
    fn guard_timeout(&self, open: &states::Open) -> bool {
        open.timer.is_timeout()
    }
    fn handle_count_reset(&mut self, _: states::Closed, _: inputs::Success) -> states::Closed {
        states::Closed { count: 0 }
    }
    fn handle_count_increment(
        &mut self,
        closed: states::Closed,
        _: inputs::Fail,
    ) -> states::Closed {
        states::Closed {
            count: closed.count + 1,
        }
    }
    fn setup_timer(&self) -> states::Open {
        states::Open {
            timer: Timer::new(self.clock.clone_box(), self.timeout),
        }
    }
    fn handle_trip_breaker(&mut self, _: states::Closed, _: inputs::Fail) -> states::Open {
        self.setup_timer()
    }
    fn handle_setup_timer(&mut self, _: states::HalfOpen, _: inputs::Fail) -> states::Open {
        self.setup_timer()
    }
}

#[test]
fn circuit_breaker() {
    let clock = ManualClock::new();
    let circuit_breaker = CircuitBreaker {
        clock: clock.clone_box(),
        threshold: 0,
        timeout: TimestampDelta::from_secs(5),
    };

    let mut cb = StateMachine::new(circuit_breaker, states::Closed::default());

    // Pass a request when the circuit breaker is closed.
    assert!(cb.can_consume::<inputs::Success>());
    assert!(cb.can_consume::<inputs::Fail>());
    cb.consume(inputs::Success);
    assert!(cb.state().is_closed());

    // Trip the circuit
    assert!(cb.can_consume::<inputs::Fail>());
    assert!(cb.can_consume::<inputs::Success>());
    cb.consume(inputs::Fail);
    assert!(cb.state().is_open());

    // Can't pass request when the circuit breaker is open.
    assert!(!cb.can_consume::<inputs::Success>());
    assert!(!cb.can_consume::<inputs::Fail>());
    // But can attempt to expire the timer
    cb.step();
    assert!(cb.state().is_open());

    // Let the timer expire
    clock.advance_by(TimestampDelta::from_secs(5));
    cb.step();
    assert!(cb.state().is_half_open());
    assert!(cb.can_consume::<inputs::Success>());
    assert!(cb.can_consume::<inputs::Fail>());

    // Pass a success
    cb.consume(inputs::Success);
    assert!(cb.state().is_closed());
}

/// An example of a web-server route that uses the circuit breaker.
///
/// See `faulty_route` test for example usage.
pub struct FaultyRoute {
    pub circuit_breaker: StateMachine<CircuitBreaker>,
}

impl FaultyRoute {
    /// Request handler that may sometimes fail, and replies based on the circuit breaker state.
    pub fn handle_request(&mut self, request: i32) -> Option<i32> {
        // Simulate that the response failed.
        let fail_response = request % 2 == 0;

        if self.circuit_breaker.state().is_closed() || self.circuit_breaker.state().is_half_open() {
            if fail_response {
                self.circuit_breaker.consume(inputs::Fail);
                None
            } else {
                self.circuit_breaker.consume(inputs::Success);
                let response = request * 3 + 1;
                Some(response)
            }
        } else {
            self.circuit_breaker.step();
            None
        }
    }
}

#[test]
pub fn faulty_route() {
    let clock = ManualClock::new();
    let timeout = TimestampDelta::from_secs(6);
    let half_timeout = TimestampDelta::from_secs(3);
    let circuit_breaker = CircuitBreaker {
        clock: clock.clone_box(),
        threshold: 0,
        timeout,
    };
    let mut faulty_route = FaultyRoute {
        circuit_breaker: StateMachine::new(circuit_breaker, states::Closed::default()),
    };
    let good_request = 1;
    let bad_request = 2;

    // Pass a request when the circuit breaker is closed.
    assert!(faulty_route.handle_request(good_request).is_some());
    assert!(faulty_route.circuit_breaker.state().is_closed());

    // Trip the circuit breaker
    assert!(faulty_route.handle_request(bad_request).is_none());
    assert!(faulty_route.circuit_breaker.state().is_open());

    // Let the timer expire
    clock.advance_by(timeout);
    assert!(faulty_route.handle_request(bad_request).is_none());
    assert!(faulty_route.circuit_breaker.state().is_half_open());

    // Pass a request when the circuit breaker is half-open.
    assert!(faulty_route.handle_request(bad_request).is_none());
    assert!(faulty_route.circuit_breaker.state().is_open());

    // Pass a request while the timer did not expire
    clock.advance_by(half_timeout);
    assert!(faulty_route.handle_request(good_request).is_none());
    assert!(faulty_route.circuit_breaker.state().is_open());

    // Let the timer expire
    clock.advance_by(half_timeout);
    assert!(faulty_route.handle_request(good_request).is_none());
    assert!(faulty_route.circuit_breaker.state().is_half_open());

    // Pass a request when the circuit breaker is half-open.
    assert!(faulty_route.handle_request(good_request).is_some());
    assert!(faulty_route.circuit_breaker.state().is_closed());
}
