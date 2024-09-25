//!
//! This module contains all the basic primitives that makes up the core of the order book

use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Order side
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub enum OrderSide {
    /// Buy side
    Buy,
    /// Sell side
    Sell,
}

/// Order type
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub enum OrderType {
    Market,
    Limit,
}

/// Order Id
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Copy, Hash)]
pub struct Oid(u64);

impl Oid {
    pub fn new(value: u64) -> Self {
        Oid(value)
    }
}

impl Display for Oid {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for Oid {
    fn from(value: u64) -> Self {
        Oid(value)
    }
}
/// Timestamp
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn new(value: u64) -> Self {
        Timestamp(value)
    }
}

impl From<chrono::DateTime<chrono::Utc>> for Timestamp {
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        Timestamp(value.timestamp_millis() as u64)
    }
}

/// Price
#[derive(Debug, Clone, Copy)]
pub struct Price(f64);

impl Price {
    pub const ZERO: Self = Price(0.0);
}

impl Eq for Price {}

impl PartialEq for Price {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Hash for Price {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for Price {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Price {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare bit patterns to handle NaN values consistently
        self.0.to_bits().cmp(&other.0.to_bits())
    }
}

impl AddAssign for Price {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Price {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Sub for Price {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Price(self.0 - rhs.0)
    }
}

impl Add for Price {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Price(self.0 + rhs.0)
    }
}

impl From<Price> for f64 {
    fn from(value: Price) -> Self {
        value.0
    }
}

impl From<f64> for Price {
    fn from(value: f64) -> Self {
        Price(value)
    }
}

/// Volume
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Volume(u64);

impl Volume {
    pub const ZERO: Self = Volume(0);

    pub fn new(value: u64) -> Self {
        Volume(value)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl From<u64> for Volume {
    fn from(value: u64) -> Self {
        Volume(value)
    }
}

impl From<Volume> for u64 {
    fn from(value: Volume) -> Self {
        value.0
    }
}

impl std::ops::AddAssign for Volume {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl std::ops::SubAssign for Volume {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl std::ops::Add for Volume {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Volume(self.0 + other.0)
    }
}

impl std::ops::Sub for Volume {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Volume(self.0 - other.0)
    }
}

impl Sum for Volume {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(0.into(), |acc, x| acc + x)
    }
}
