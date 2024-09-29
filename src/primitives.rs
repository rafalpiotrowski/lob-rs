//!
//! This module contains all the basic primitives that makes up the core of the order book

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Deref, DerefMut, Sub, SubAssign};

/// Spread
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Spread(pub f64);

impl From<f64> for Spread {
    fn from(value: f64) -> Self {
        Spread(value)
    }
}

impl From<Spread> for f64 {
    fn from(value: Spread) -> Self {
        value.0
    }
}

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
pub struct Price(pub f64);

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

/// LevelIndex is an index to a Level in a stable vec
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LevelIndex(pub usize);

impl From<usize> for LevelIndex {
    fn from(value: usize) -> Self {
        LevelIndex(value)
    }
}

impl From<LevelIndex> for usize {
    fn from(value: LevelIndex) -> Self {
        value.0
    }
}

impl<'a> From<&'a LevelIndex> for &'a usize {
    fn from(value: &'a LevelIndex) -> Self {
        &value.0
    }
}

impl Deref for LevelIndex {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LevelIndex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// map of Limit -> LevelIndex
// this will allow for O(1) lookup of Limit levels
// this will only grow, since each limit need to point to a stable index in the stable level vec
#[derive(Debug, Clone, Default)]
pub struct LevelMap(pub HashMap<Price, LevelIndex>);

impl Deref for LevelMap {
    type Target = HashMap<Price, LevelIndex>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LevelMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// map of Order ID -> LimitOrder that contains full order data
#[derive(Debug, Default)]
pub struct OrderMap(pub HashMap<Oid, LimitOrder>);
impl Deref for OrderMap {
    type Target = HashMap<Oid, LimitOrder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OrderMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Order
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Order {
    pub id: Oid,
    pub side: OrderSide,
    pub kind: OrderType,
    pub price: Option<Price>,
    pub volume: Volume,
    pub timestamp: Timestamp,
}

impl Order {
    /// Create a new order
    pub fn new_limit(
        id: Oid,
        side: OrderSide,
        timestamp: Timestamp,
        price: Price,
        volume: Volume,
    ) -> Self {
        Order {
            id,
            side,
            kind: OrderType::Limit,
            timestamp,
            price: Some(price),
            volume,
        }
    }
    pub fn new_market(id: Oid, side: OrderSide, timestamp: Timestamp, volume: Volume) -> Self {
        Order {
            id,
            side,
            kind: OrderType::Market,
            timestamp,
            price: None,
            volume,
        }
    }
}

impl TryInto<LimitOrder> for Order {
    type Error = TryFromOrderError;

    fn try_into(self) -> Result<LimitOrder, Self::Error> {
        match self.kind {
            OrderType::Limit => Ok(LimitOrder {
                id: self.id,
                side: self.side,
                timestamp: self.timestamp,
                price: self.price.unwrap(), // we can unwrap since we know it is a limit order
                volume: self.volume,
                filled_volume: None,
            }),
            _ => Err(TryFromOrderError::OrderTypeNotLimit),
        }
    }
}

/// Limit Order
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct LimitOrder {
    pub id: Oid,
    pub side: OrderSide,
    pub timestamp: Timestamp,
    pub price: Price,
    pub volume: Volume,
    pub filled_volume: Option<Volume>,
}

#[derive(Debug)]
pub enum TryFromOrderError {
    OrderTypeNotLimit,
}

impl TryFrom<&Order> for LimitOrder {
    type Error = TryFromOrderError;

    fn try_from(order: &Order) -> Result<Self, Self::Error> {
        match order.kind {
            OrderType::Limit => Ok(LimitOrder {
                id: order.id,
                side: order.side,
                timestamp: order.timestamp,
                price: order.price.unwrap(), // we can unwrap since we know it is a limit order
                volume: order.volume,
                filled_volume: None,
            }),
            _ => Err(TryFromOrderError::OrderTypeNotLimit),
        }
    }
}

impl LimitOrder {
    /// Create a new order
    pub fn new(
        id: Oid,
        side: OrderSide,
        timestamp: Timestamp,
        price: Price,
        volume: Volume,
    ) -> Self {
        LimitOrder {
            id,
            side,
            timestamp,
            price,
            volume,
            filled_volume: None,
        }
    }
}
