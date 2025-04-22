//! Provide structs for measuring time.

use crate::timestamp::Timestamp;
use crate::timestamp::TimestampDelta;
use std::fmt;

/// A trait for providing the current time.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
    fn clone_box(&self) -> Box<dyn Clock>;
}

/// A time provider that uses the system's clock.
#[derive(Clone, Debug)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        // We use chrono here as it is platform agnostic.
        Timestamp::from(chrono::Utc::now())
    }
    fn clone_box(&self) -> Box<dyn Clock> {
        Box::new(self.clone())
    }
}

use std::sync::{Arc, Mutex};

/// A time provider that can be mocked to advance time.
#[derive(Clone, Debug, Default)]
pub struct ManualClock {
    current_time: Arc<Mutex<Timestamp>>,
}

impl ManualClock {
    pub fn new() -> Self {
        let zero_time = Timestamp::zero();
        Self {
            current_time: Arc::new(Mutex::new(zero_time)),
        }
    }

    pub fn advance_by(&self, duration: TimestampDelta) {
        assert!(duration > TimestampDelta::zero());
        let mut time = self.current_time.lock().unwrap();
        *time = *time + duration;
    }

    pub fn advance_to(&self, time: Timestamp) {
        let mut current_time = self.current_time.lock().unwrap();
        *current_time = time;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        *self.current_time.lock().unwrap()
    }

    fn clone_box(&self) -> Box<dyn Clock> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn advance_time_in_mocked_time_provider() {
        struct Component {
            times: Vec<Timestamp>,
            provider: Box<dyn Clock>,
        }

        impl Component {
            fn new(provider: Box<dyn Clock>) -> Self {
                Self {
                    times: Vec::new(),
                    provider,
                }
            }
            fn append_now(&mut self) {
                self.times.push(self.provider.now());
            }
        }

        let clock = ManualClock::new();
        let mut a = Component::new(clock.clone_box());
        let mut b = Component::new(clock.clone_box());

        a.append_now(); // t=0
        clock.advance_by(TimestampDelta::from_secs(1)); // t=0 -> t=1
        a.append_now(); // t=1
        b.append_now(); // t=1
        clock.advance_by(TimestampDelta::from_secs(2)); // t=1 -> t=3
        a.append_now(); // t=3
        clock.advance_by(TimestampDelta::from_secs(1)); // t=3 -> t=4
        a.append_now(); // t=4
        b.append_now(); // t=4

        assert_eq!(
            a.times.iter().map(|t| t.as_secs()).collect::<Vec<_>>(),
            vec![0, 1, 3, 4]
        );
        assert_eq!(
            b.times.iter().map(|t| t.as_secs()).collect::<Vec<_>>(),
            vec![1, 4]
        );
    }

    #[test]
    fn advance_time_across_threads_simplified() {
        use std::sync::{mpsc::sync_channel, Arc, Barrier};

        let clock = ManualClock::new();
        let worker_count = 4;
        let steps = 4;

        // A barrier that synchronizes the main thread plus all worker threads.
        // Use two-phase barrier synchronization.
        let barrier = Arc::new(Barrier::new(worker_count + 1));
        let (sender, receiver) = sync_channel(worker_count);
        for _ in 0..worker_count {
            let clock = clock.clone_box();
            let barrier = barrier.clone();
            let sender = sender.clone();

            std::thread::spawn(move || {
                let mut times = Vec::with_capacity(steps);
                for _ in 0..steps {
                    barrier.wait(); // phase one: ensure that the shared state is ready.
                    times.push(clock.now()); // record the current time
                    barrier.wait(); // phase two: ensure that all threads have completed their work.
                }
                sender.send(times).unwrap();
            });
        }

        for _ in 0..steps {
            clock.advance_by(TimestampDelta::from_secs(1));
            barrier.wait(); // let worker threads read the updated time
            barrier.wait(); // wait until they finish recording before next iteration
        }

        // Collect and check results.
        let results: Vec<Vec<Timestamp>> = (0..worker_count)
            .map(|_| receiver.recv().unwrap().into_iter().collect())
            .collect();

        for res in results {
            assert_eq!(
                res,
                (1..steps + 1)
                    .map(|i| Timestamp::from_secs(i as i64))
                    .collect::<Vec<_>>(),
                "All thread components should show consistent time steps"
            );
        }
    }
}

/// Measure elapsed time.
pub struct Stopwatch {
    clock: Box<dyn Clock>,
    start_time: Timestamp,
}

impl fmt::Debug for Stopwatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stopwatch")
            .field("clock", &self.clock.now()) // Just show a placeholder
            .field("start_time", &self.start_time)
            .finish()
    }
}

impl Stopwatch {
    pub fn new(clock: Box<dyn Clock>) -> Self {
        Self {
            start_time: clock.now(),
            clock,
        }
    }

    pub fn elapsed(&self) -> TimestampDelta {
        self.clock.now() - self.start_time
    }

    pub fn reset(&mut self) {
        self.start_time = self.clock.now();
    }
}

/// A timer that can be used to measure the elapsed time and check if timeout has occurred.
#[derive(Debug)]
pub struct Timer {
    stopwatch: Stopwatch,
    delay: TimestampDelta,
}

impl Timer {
    pub fn new(clock: Box<dyn Clock>, delay: TimestampDelta) -> Self {
        Self {
            delay,
            stopwatch: Stopwatch::new(clock),
        }
    }

    pub fn is_timeout(&self) -> bool {
        self.stopwatch.elapsed() >= self.delay
    }

    pub fn elapsed(&self) -> TimestampDelta {
        self.stopwatch.elapsed()
    }

    pub fn reset(&mut self) {
        self.stopwatch.reset();
    }
}
