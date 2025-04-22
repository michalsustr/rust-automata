//! Provide timestamp and timestamp delta types.
//!
//! Useful for internal representation of time, and exposes methods for conversion to and from `DateTime`.
use chrono::{DateTime, Local, TimeDelta, TimeZone, Utc};
use core::fmt;
use std::fmt::Display;
use std::num::ParseIntError;
use std::ops::{Add, Sub};
use std::str::FromStr;

/// A timestamp in nanoseconds in the UTC timezone.
///
/// Use this type for internal timestamps and for nice date formatting
/// use [`DateTime<Local>`].
///
/// The dates that can be represented as nanoseconds are between
/// 1677-09-21T00:12:43.145224192 and 2262-04-11T23:47:16.854775807.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Timestamp(i64);

/// A timestamp delta (duration) in nanoseconds.
///
/// Any time you subtract two timestamps, you get a `TimestampDelta`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct TimestampDelta(i64);

impl Timestamp {
    pub const fn zero() -> Self {
        Self(0)
    }
    pub const fn as_secs(&self) -> i64 {
        self.0 / 1_000_000_000
    }
    pub const fn as_millis(&self) -> i64 {
        self.0 / 1_000_000
    }
    pub const fn as_micros(&self) -> i64 {
        self.0 / 1_000
    }
    pub const fn as_nanos(&self) -> i64 {
        self.0
    }
    pub fn local(&self) -> DateTime<Local> {
        DateTime::<Local>::from(*self)
    }
    pub fn utc(&self) -> DateTime<Utc> {
        DateTime::<Utc>::from(*self)
    }

    // TODO: bounds check
    pub const fn from_hours(hours: i64) -> Self {
        Self(hours * 60 * 60 * 1_000_000_000)
    }
    pub const fn from_minutes(minutes: i64) -> Self {
        Self(minutes * 60 * 1_000_000_000)
    }
    pub const fn from_secs(secs: i64) -> Self {
        Self(secs * 1_000_000_000)
    }
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis * 1_000_000)
    }
    pub const fn from_micros(micros: i64) -> Self {
        Self(micros * 1_000)
    }
    pub const fn from_nanos(nanos: i64) -> Self {
        Self(nanos)
    }
}

impl TimestampDelta {
    pub const fn zero() -> Self {
        Self(0)
    }
    pub const fn as_secs(&self) -> i64 {
        self.0 / 1_000_000_000
    }
    pub const fn as_millis(&self) -> i64 {
        self.0 / 1_000_000
    }
    pub const fn as_micros(&self) -> i64 {
        self.0 / 1_000
    }
    pub const fn as_nanos(&self) -> i64 {
        self.0
    }

    // TODO: bounds check
    pub const fn from_hours(hours: i64) -> Self {
        Self(hours * 60 * 60 * 1_000_000_000)
    }
    pub const fn from_minutes(minutes: i64) -> Self {
        Self(minutes * 60 * 1_000_000_000)
    }
    pub const fn from_secs(secs: i64) -> Self {
        Self(secs * 1_000_000_000)
    }
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis * 1_000_000)
    }
    pub const fn from_micros(micros: i64) -> Self {
        Self(micros * 1_000)
    }
    pub const fn from_nanos(nanos: i64) -> Self {
        Self(nanos)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for Timestamp {
    fn from(nanos: i64) -> Self {
        Self(nanos)
    }
}

impl Add<TimeDelta> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: TimeDelta) -> Self::Output {
        Timestamp::from(self.0 + rhs.num_nanoseconds().unwrap())
    }
}
impl Add<Timestamp> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: Timestamp) -> Self::Output {
        Timestamp::from(self.0 + rhs.0)
    }
}
impl Add<TimestampDelta> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: TimestampDelta) -> Self::Output {
        Timestamp::from(self.0 + rhs.0)
    }
}

impl Sub<TimeDelta> for Timestamp {
    type Output = TimestampDelta;

    fn sub(self, rhs: TimeDelta) -> Self::Output {
        TimestampDelta::from(self.0 - rhs.num_nanoseconds().unwrap())
    }
}
impl Sub<Timestamp> for Timestamp {
    type Output = TimestampDelta;

    fn sub(self, rhs: Timestamp) -> Self::Output {
        TimestampDelta::from(self.0 - rhs.0)
    }
}

impl FromStr for Timestamp {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let nanos = i64::from_str(s)?;
        Ok(Timestamp::from(nanos))
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt.timestamp_nanos_opt().unwrap())
    }
}

impl From<DateTime<Local>> for Timestamp {
    fn from(dt: DateTime<Local>) -> Self {
        Self(dt.with_timezone(&Utc).timestamp_nanos_opt().unwrap())
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(ts: Timestamp) -> Self {
        Utc.timestamp_nanos(ts.0)
    }
}

impl From<Timestamp> for DateTime<Local> {
    fn from(ts: Timestamp) -> Self {
        let utc: DateTime<Utc> = ts.into();
        utc.with_timezone(&Local)
    }
}

impl From<Timestamp> for TimeDelta {
    fn from(ts: Timestamp) -> Self {
        TimeDelta::nanoseconds(ts.0)
    }
}

impl Display for TimestampDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for TimestampDelta {
    fn from(nanos: i64) -> Self {
        Self(nanos)
    }
}

impl Add<TimeDelta> for TimestampDelta {
    type Output = TimestampDelta;

    fn add(self, rhs: TimeDelta) -> Self::Output {
        TimestampDelta::from(self.0 + rhs.num_nanoseconds().unwrap())
    }
}
impl Add<TimestampDelta> for TimestampDelta {
    type Output = TimestampDelta;

    fn add(self, rhs: TimestampDelta) -> Self::Output {
        TimestampDelta::from(self.0 + rhs.0)
    }
}

impl Sub<TimeDelta> for TimestampDelta {
    type Output = TimestampDelta;

    fn sub(self, rhs: TimeDelta) -> Self::Output {
        TimestampDelta::from(self.0 - rhs.num_nanoseconds().unwrap())
    }
}
impl Sub<TimestampDelta> for TimestampDelta {
    type Output = TimestampDelta;

    fn sub(self, rhs: TimestampDelta) -> Self::Output {
        TimestampDelta::from(self.0 - rhs.0)
    }
}

impl From<TimeDelta> for TimestampDelta {
    fn from(delta: TimeDelta) -> Self {
        TimestampDelta::from(delta.num_nanoseconds().unwrap())
    }
}

impl From<TimestampDelta> for TimeDelta {
    fn from(delta: TimestampDelta) -> Self {
        TimeDelta::nanoseconds(delta.0)
    }
}
