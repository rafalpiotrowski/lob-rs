//!
//! A limit order book is a record of outstanding limit orders maintained by the security specialist
//! who works at the exchange. The book is organized by Limit level and contains the Limit and
//! volume of each limit order. The specialist is responsible for maintaining a fair and orderly
//! market in the security and uses the book to help determine the best Limit at which to execute
//! orders.
//!
//! The limit order book is a key component of the market microstructure and is used by traders to
//! help make trading decisions. The book is also used by the exchange to help determine the best
//! Limit at which to execute orders. The book is updated in real-time as orders are placed and
//! executed.
//!

mod primitives;
use itertools::Itertools;
use stable_vec::StableVec;
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};
use thiserror::Error;

pub use primitives::{
    LimitOrder, Oid, Order, OrderSide, OrderType, Price, Spread, Timestamp, Volume,
};

use primitives::{LevelIndex, LevelMap, OrderMap};

/// Limit level
/// represents Price level and list of orders in FIFO order
#[derive(Debug, Clone)]
pub struct Level {
    index: Option<LevelIndex>,
    price: Price,
    total_volume: Volume,
    orders: VecDeque<Oid>,
    depth: usize, // number of orders at the level
}

impl Eq for Level {}
impl PartialEq for Level {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}

impl PartialOrd for Level {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Level {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.price.cmp(&other.price)
    }
}

impl Level {
    /// Create a new Limit level
    pub fn new(price: Price) -> Level {
        Level {
            index: None,
            price,
            total_volume: 0.into(),
            orders: VecDeque::new(),
            depth: 0,
        }
    }

    /// Add an order to the Limit level
    pub fn add_order(&mut self, order: &LimitOrder) {
        {
            self.total_volume += order.volume;
            self.depth += 1;
        }
        self.orders.push_back(order.id);
    }

    /// Remove an order from the Limit level
    /// order will be cancelled and removed from the map
    /// therefore we need to update the total volume
    pub fn cancell_order(&mut self, order: &LimitOrder) {
        self.total_volume -= order.volume;
        self.depth -= 1;
    }
}

// stable vec of levels, once added level will not change its index
// it will be removed only when the level is empty
// so when looking up the index we will get None
#[derive(Debug, Clone, Default)]
struct Levels(StableVec<Level>);

impl Levels {
    fn push(&mut self, level: Level) -> LevelIndex {
        LevelIndex(self.0.push(level))
    }

    fn get(&self, index: LevelIndex) -> Option<&Level> {
        self.0.get(*index)
    }

    fn get_mut(&mut self, index: LevelIndex) -> Option<&mut Level> {
        self.0.get_mut(*index)
    }
}

impl Deref for Levels {
    type Target = StableVec<Level>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Levels {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Limits (i.e. Price): 21.0453 to orders at that price
#[derive(Debug, Default)]
pub struct Limits {
    /// map of LimitIndex -> Level (i.e. queue of orders at that Limit level)
    /// this will allow for O(1) lookup of Limit levels
    /// when inserting an order at a specific Limit level
    levels: Levels,
    // stable vector of Limit levels
    // once inserted limit will not change its index
    // and hence we will be able to make O(1) lookup to access orders at that level
    level_map: LevelMap,
    // for bids is max for asks is min limit
    best: Option<LevelIndex>,
}

impl Limits {
    /// depends on the side, i.e. for ask find smallest Limit, for bid find largest Limit
    pub fn get_best_limit(&self) -> Option<Price> {
        if let Some(index) = self.best {
            self.levels.get(index).map(|l| l.price)
        } else {
            None
        }
    }

    /// add an order to the Limit map
    pub fn add_order(&mut self, order: &LimitOrder) {
        let price = &order.price;
        match self.level_map.get(price) {
            None => {
                // create a new limit level
                let mut level = Level::new(*price);
                level.add_order(order);
                let index = self.levels.push(level);
                let level = self.levels.get_mut(index).unwrap();
                level.index = Some(index);
                self.level_map.insert(*price, index.into());

                // update the best limit
                if let Some(current_best_index) = self.best {
                    if let Some(best_level) = self.levels.get(current_best_index) {
                        match order.side {
                            OrderSide::Buy => {
                                if *price > best_level.price {
                                    self.best = Some(index);
                                }
                            }
                            OrderSide::Sell => {
                                if *price < best_level.price {
                                    self.best = Some(index);
                                }
                            }
                        }
                    }
                } else {
                    self.best = Some(index);
                }
            }
            Some(index) => {
                // add the order to the existing Limit level
                if let Some(level) = self.levels.get_mut(*index) {
                    level.add_order(order);
                }
                // no need to check for best limit since we are adding to existing level
            }
        }
    }

    /// cancell order
    /// since we postopne removal of cancelled orders when filling the new order
    /// all we need to do is to update the total level volume so it is in sync
    pub fn cancel_order(&mut self, order: &LimitOrder) {
        if let Some(index) = self.level_map.get(&order.price) {
            if let Some(level) = self.levels.get_mut(*index) {
                level.cancell_order(order);
            }
        }
    }
}

/// Place order error
#[derive(Error, Debug, PartialEq, PartialOrd, Clone)]
pub enum PlaceOrderError {
    /// Order cannot be placed
    #[error("Order cannot be placed: {0}")]
    OrderCannotBePlaced(String),
}

/// Trade
#[derive(Debug)]
#[allow(dead_code)]
pub struct Trade {
    order_id: Oid,
    volume: Volume,
    filled_volume: Volume,
    executions: Vec<Execution>,
}

impl Trade {
    /// Create a new trade
    pub fn new(order_id: Oid, volume: Volume) -> Self {
        Trade {
            order_id,
            volume,
            filled_volume: Volume::ZERO,
            executions: Vec::new(),
        }
    }

    /// Add an execution to the trade
    pub fn add_execution(&mut self, execution: Execution) {
        self.filled_volume += execution.volume;
        self.executions.push(execution)
    }
}

/// Execution
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Execution {
    order_id: Oid,
    price: Price,
    volume: Volume,
}

impl Execution {
    /// Create a new execution
    pub fn new(order_id: Oid, price: Price, volume: Volume) -> Self {
        Execution {
            order_id,
            price,
            volume,
        }
    }
}

/// Cancellation status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancellationStatus {
    /// Order was cancelled
    Cancelled,
    /// Order was not cancelled
    NotCancelled(String),
}

/// Cancellation report
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CancellationReport {
    order_id: Oid,
    status: CancellationStatus,
}

/// Cancel order error  
#[derive(Error, Debug, PartialEq, PartialOrd, Clone)]
pub enum CancelOrderError {
    /// Order not found
    #[error("Order {0} not found")]
    NotFound(Oid),
    /// Order already cancelled
    #[error("Order {0} already cancelled")]
    AlreadyCancelled(Oid),
}

#[derive(Error, Debug, PartialEq, PartialOrd, Clone)]
pub enum FillError {}

/// Limit Order Book
/// Trades are made when highest bid Limit is greater than or equal to the lowest ask Limit (spread is crossed)
/// If order cannot be filled immediately, it is added to the book
#[derive(Debug, Default)]
pub struct OrderBook {
    // Bid side of the book, represents open offers to buy an asset
    bids: Limits,
    // Ask side of the book, represents open offers to sell an asset
    asks: Limits,
    // this will allow for O(1) lookup of orders for cancellation
    orders: OrderMap,
    // spread is the diff between min ask and max bid
    spread: Option<Spread>,
}

impl OrderBook {
    pub fn add_order(&mut self, order: LimitOrder) {
        match order.side {
            OrderSide::Buy => self.bids.add_order(&order),
            OrderSide::Sell => self.asks.add_order(&order),
        }
    }

    pub fn update_spreads(&mut self) {
        let ask_best_limit = self.asks.get_best_limit();
        let bid_best_limit = self.bids.get_best_limit();
        match (ask_best_limit, bid_best_limit) {
            (Some(ask_limit), Some(bid_limit)) => {
                self.spread = Some(Spread((ask_limit - bid_limit).into()));
            }
            _ => {
                self.spread = None;
            }
        }
    }

    pub fn update_best_limits(&mut self) {
        self.update_best_buy();
        self.update_best_sell();
    }

    fn update_best_buy(&mut self) {
        if let Some(max) = self
            .bids
            .levels
            .values()
            .filter(|l| l.total_volume > 0.into())
            .max()
        {
            self.bids.best = self.bids.level_map.get(&max.price).copied();
        }
    }

    fn update_best_sell(&mut self) {
        if let Some(min) = self
            .asks
            .levels
            .values()
            .filter(|l| l.total_volume > 0.into())
            .min()
        {
            self.asks.best = self.asks.level_map.get(&min.price).copied();
        }
    }

    pub fn get_best_sell(&self) -> Option<Price> {
        self.asks.get_best_limit()
    }
    pub fn get_best_buy(&self) -> Option<Price> {
        self.bids.get_best_limit()
    }

    /// cancellation does not modify any of the underlying collections. Order is marked as cancelled and will be removed
    /// at the time of order filling, when we iterate over the orders
    pub fn cancel_order(&mut self, order_id: Oid) -> Result<CancellationReport, CancelOrderError> {
        // immutable borrows of self, therefore the need for new scope
        // so if we do not return err then the immutable borrow will go out of scope
        // and will allow for mutable borrow to allow for removal of the order from hashmap
        match self.orders.remove(&order_id) {
            None => return Err(CancelOrderError::NotFound(order_id)),
            Some(order) => {
                // update the level so the level volume is updated
                match order.side {
                    OrderSide::Buy => self.bids.cancel_order(&order),
                    OrderSide::Sell => self.asks.cancel_order(&order),
                }
            }
        }
        Ok(CancellationReport {
            order_id,
            status: CancellationStatus::Cancelled,
        })
    }

    /// get volume of open orders for either buying or selling side of the book
    pub fn get_volume_at_limit(&self, limit: Price, side: OrderSide) -> Option<Volume> {
        let limit_map = match side {
            OrderSide::Buy => &self.bids,
            OrderSide::Sell => &self.asks,
        };
        limit_map
            .level_map
            .get(&limit)
            .map(|index| limit_map.levels[**index].total_volume)
    }

    pub fn fill_buy_order(
        &mut self,
        mut trade: Trade,
        buy_price: Option<Price>,
    ) -> Result<Trade, PlaceOrderError> {
        // find the lowest sell Limit
        // if the lowest sell Limit is less than or equal to the buy Limit, we can fill the order, substracting the volume
        // if the lowest sell Limit is greater than the buy Limit, we add the order to the book, with the volume
        // equal to the order quantity

        // before we do sorting we fill against best sell
        if let Some(best_sell_level_index) = self.asks.best {
            self.fill_buy_order_from_level(&mut trade, best_sell_level_index);

            if trade.filled_volume == trade.volume {
                let best_sell_level = self.asks.levels.get_mut(best_sell_level_index).unwrap();
                if best_sell_level.orders.is_empty() {
                    self.update_best_sell();
                }
                return Ok(trade);
            }
        }

        // if we still have something to fill, we do not need to update best sell now, we will do it later
        // when we finish filling the order

        let sorted = self
            .asks
            .levels
            .values_mut()
            .filter(|l| filter_limit_for_buy(l, &buy_price))
            .sorted();

        let mut remaining_buy_volume = trade.volume - trade.filled_volume;

        'top: for l in sorted {
            // update best sell
            // this will keep updating best index with each iteration
            if self.bids.best != l.index {
                self.bids.best = l.index;
            }
            // peek order at front of the level
            while let Some(oid) = l.orders.front() {
                // todo: remove might trigger memcpy
                // although we need to get the owned value otherwise we will be borrowing self hence problem with borrow checker
                let Some(mut sell_order) = self.orders.remove(oid) else {
                    // if there is no order then it might have been cancelled
                    // and removed from the map, and since we pospone the removal of orders from the level
                    // till we encounter such order, we can safely remove the order from the level
                    l.orders.pop_front();
                    continue;
                };
                let sell_volume = sell_order.volume;
                if sell_volume <= remaining_buy_volume {
                    // fill the sell order
                    trade.add_execution(Execution::new(
                        sell_order.id,
                        sell_order.price,
                        sell_volume,
                    ));
                    // remove order from the level
                    l.orders.pop_front();
                    l.cancell_order(&sell_order);
                    sell_order.volume = Volume::ZERO;
                    remaining_buy_volume -= sell_volume;
                } else {
                    // fill the buy order, put the order back to the book
                    let execution =
                        Execution::new(sell_order.id, sell_order.price, remaining_buy_volume);
                    trade.add_execution(execution);
                    sell_order.volume -= remaining_buy_volume;
                    remaining_buy_volume = Volume::ZERO;
                }
                // we should put back the sell order if it was not completely filled
                if !sell_order.volume.is_zero() {
                    self.orders.insert(sell_order.id, sell_order);
                }
                // if buy order was filled completely, we can break the loop
                if remaining_buy_volume.is_zero() {
                    break 'top;
                }
                // otherwise we still have volume to fill
            } // no more orders at the level, we can move to the next level
        }
        Ok(trade)
    }

    fn fill_buy_order_from_level(&mut self, trade: &mut Trade, sell_level_index: LevelIndex) {
        let sell_level = self.asks.levels.get_mut(sell_level_index).unwrap();

        let mut remaining_buy_volume = trade.volume;
        // peek order at front of the level
        while let Some(sell_order_oid) = sell_level.orders.front() {
            let Some(mut sell_order) = self.orders.remove(sell_order_oid) else {
                // if there is no order then it might have been cancelled
                // and removed from the map, and since we pospone the removal of orders from the level
                // till we encounter such order, we can safely remove the order from the level
                sell_level.orders.pop_front();
                continue;
            };
            let sell_volume = sell_order.volume;
            if sell_volume <= remaining_buy_volume {
                // fill the sell order
                trade.add_execution(Execution::new(sell_order.id, sell_order.price, sell_volume));
                // remove order from the level
                sell_level.orders.pop_front();
                sell_level.cancell_order(&sell_order);
                sell_order.volume = Volume::ZERO;
                remaining_buy_volume -= sell_volume;
            } else {
                // sell_volume > remaining_buy_volume
                // fill the sell order, put the order back to the book
                let execution =
                    Execution::new(sell_order.id, sell_order.price, remaining_buy_volume);
                trade.add_execution(execution);
                sell_order.volume -= remaining_buy_volume;
                remaining_buy_volume = Volume::ZERO;
            }
            // we should put back the sell order if it was not completely filled
            if !sell_order.volume.is_zero() {
                self.orders.insert(sell_order.id, sell_order);
            }
            // if buy order was filled completely, we can break the loop
            if remaining_buy_volume.is_zero() {
                break;
            }
        }
    }

    pub fn fill_sell_order(
        &mut self,
        mut trade: Trade,
        sell_price: Option<Price>,
    ) -> Result<Trade, PlaceOrderError> {
        // find the highest bid Limit
        // if the highest bid Limit is greater than or equal to the ask Limit, we can fill the order, substracting the volume
        // if the highest bid Limit is less than the ask Limit, we add the order to the book, with the volume
        // equal to the order quantity

        // before we do sorting we fill against best sell
        if let Some(best_buy_level_index) = self.bids.best {
            self.fill_sell_order_from_level(&mut trade, best_buy_level_index);

            if trade.filled_volume == trade.volume {
                let best_buy_level = self.bids.levels.get_mut(best_buy_level_index).unwrap();
                if best_buy_level.orders.is_empty() {
                    self.update_best_sell();
                }
                return Ok(trade);
            }
        }

        let mut remaining_sell_volume = trade.volume;

        let sorted = self
            .bids
            .levels
            .values_mut()
            .filter(|l| filter_limit_for_sell(l, &sell_price))
            .sorted_by(sort_limit_descending);

        'top: for l in sorted {
            // update best sell
            // this will keep updating best index with each iteration
            if self.asks.best != l.index {
                self.asks.best = l.index;
            }
            // peek order at front of the level
            while let Some(oid) = l.orders.front() {
                // todo: remove might trigger memcpy
                // although we need to get the owned value otherwise we will be borrowing self hence problem with borrow checker
                let Some(mut buy_order) = self.orders.remove(oid) else {
                    // if there is no order then it might have been cancelled
                    // and removed from the map, and since we pospone the removal of orders from the level
                    // till we encounter such order, we can safely remove the order from the level
                    l.orders.pop_front();
                    continue;
                };
                let buy_volume = buy_order.volume;
                if buy_volume <= remaining_sell_volume {
                    // fill the sell order
                    trade.add_execution(Execution::new(buy_order.id, buy_order.price, buy_volume));
                    // remove order from the level
                    l.orders.pop_front();
                    l.cancell_order(&buy_order);
                    buy_order.volume = Volume::ZERO;
                    remaining_sell_volume -= buy_volume;
                } else {
                    // fill the buy order, put the order back to the book
                    let execution =
                        Execution::new(buy_order.id, buy_order.price, remaining_sell_volume);
                    trade.add_execution(execution);
                    buy_order.volume -= remaining_sell_volume;
                    remaining_sell_volume = Volume::ZERO;
                }
                // we should put back the sell order if it was not completely filled
                if !buy_order.volume.is_zero() {
                    self.orders.insert(buy_order.id, buy_order);
                }
                // if sell order was filled completely, we can break the loop
                if remaining_sell_volume.is_zero() {
                    break 'top;
                }
                // otherwise we still have volume to fill
            }
        }
        Ok(trade)
    }

    fn fill_sell_order_from_level(&mut self, trade: &mut Trade, buy_level_index: LevelIndex) {
        let buy_level = self.bids.levels.get_mut(buy_level_index).unwrap();

        let mut remaining_sell_volume = trade.volume;
        // peek order at front of the level
        while let Some(buy_order_oid) = buy_level.orders.front() {
            let Some(mut buy_order) = self.orders.remove(buy_order_oid) else {
                // if there is no order then it might have been cancelled
                // and removed from the map, and since we pospone the removal of orders from the level
                // till we encounter such order, we can safely remove the order from the level
                buy_level.orders.pop_front();
                continue;
            };
            let buy_volume = buy_order.volume;
            if buy_volume <= remaining_sell_volume {
                // fill the sell order
                trade.add_execution(Execution::new(buy_order.id, buy_order.price, buy_volume));
                // remove order from the level
                buy_level.orders.pop_front();
                buy_level.cancell_order(&buy_order);
                buy_order.volume = Volume::ZERO;
                remaining_sell_volume -= buy_volume;
            } else {
                // sell_volume > remaining_buy_volume
                // fill the sell order, put the order back to the book
                let execution =
                    Execution::new(buy_order.id, buy_order.price, remaining_sell_volume);
                trade.add_execution(execution);
                buy_order.volume -= remaining_sell_volume;
                remaining_sell_volume = Volume::ZERO;
            }
            // we should put back the sell order if it was not completely filled
            if !buy_order.volume.is_zero() {
                self.orders.insert(buy_order.id, buy_order);
            }
            // if buy order was filled completely, we can break the loop
            if remaining_sell_volume.is_zero() {
                break;
            }
        }
    }
}

// we want to inline since this is a small function and we want to avoid the overhead of a function call
#[inline]
#[allow(clippy::needless_lifetimes)]
fn sort_limit_descending<'a, 'b>(l: &'a &mut Level, r: &'b &mut Level) -> std::cmp::Ordering {
    l.price.cmp(&r.price).reverse()
}
#[inline]
#[allow(clippy::needless_lifetimes)]
fn filter_limit_for_buy<'a>(l: &'a &mut Level, price: &Option<Price>) -> bool {
    if l.total_volume > 0.into() {
        // in case price is none, we want to return true since we are in market order which has no price
        return price.map(|p| l.price <= p).unwrap_or(true);
    }
    false
}
#[inline]
#[allow(clippy::needless_lifetimes)]
fn filter_limit_for_sell<'a>(l: &'a &mut Level, price: &Option<Price>) -> bool {
    if l.total_volume > 0.into() {
        // in case price is none, we want to return true since we are in market order which has no price
        return price.map(|p| l.price >= p).unwrap_or(true);
    }
    false
}

mod tests_limit_map {

    #[test]
    fn test_limit_map() {
        let mut limit_map = crate::Limits::default();
        let order = crate::LimitOrder::new(
            crate::primitives::Oid::new(1),
            crate::OrderSide::Buy,
            crate::primitives::Timestamp::new(1),
            21.0453.into(),
            100.into(),
        );
        limit_map.add_order(&order);
    }
}

mod tests_order_book {

    #[test]
    fn test_order_book_new() {
        let order_book = crate::OrderBook::default();
        assert_eq!(order_book.bids.best, None);
        assert_eq!(order_book.asks.best, None);
        assert_eq!(order_book.orders.len(), 0);
        assert_eq!(order_book.spread, None);
    }

    // #[test]
    // fn test_cancel_order() {
    //     let mut order_book = crate::OrderBook::default();
    //     let order = &crate::Order::new_limit(
    //         crate::primitives::Oid::new(1),
    //         crate::OrderSide::Buy,
    //         chrono::Utc::now().into(),
    //         21.0453.into(),
    //         100.into(),
    //     );
    //     let _ = order_book.execute(order);
    //     assert_eq!(order_book.orders.len(), 1);
    //     let order = order_book
    //         .cancel_order(crate::primitives::Oid::new(1))
    //         .unwrap();
    //     assert_eq!(order_book.orders.len(), 0);
    //     assert_eq!(order.order_id, crate::primitives::Oid::new(1));
    //     assert_eq!(order.status, crate::CancellationStatus::Cancelled);

    //     let order = &crate::Order::new_limit(
    //         crate::primitives::Oid::new(2),
    //         crate::OrderSide::Buy,
    //         chrono::Utc::now().into(),
    //         21.0453.into(),
    //         50.into(),
    //     );
    //     let _ = order_book.execute(order);
    //     assert_eq!(order_book.orders.len(), 1);
    //     let order = order_book
    //         .cancel_order(crate::primitives::Oid::new(2))
    //         .unwrap();
    //     assert_eq!(order_book.orders.len(), 0);
    //     assert_eq!(order.order_id, crate::primitives::Oid::new(2));
    //     assert_eq!(order.status, crate::CancellationStatus::Cancelled);
    // }

    // #[test]
    // fn test_execute_buy_order() {
    //     let mut order_book = crate::OrderBook::default();
    //     let order = &crate::Order::new_limit(
    //         crate::primitives::Oid::new(1),
    //         crate::OrderSide::Sell,
    //         chrono::Utc::now().into(),
    //         21.0453.into(),
    //         100.into(),
    //     );
    //     let trade = order_book.execute(order).unwrap();
    //     assert_eq!(trade.order_id, crate::primitives::Oid::new(1));
    //     assert_eq!(trade.volume, 100.into());
    //     assert_eq!(trade.filled_volume, 0.into());
    //     assert_eq!(trade.executions.len(), 0);

    //     let order = &crate::Order::new_limit(
    //         crate::primitives::Oid::new(3),
    //         crate::OrderSide::Sell,
    //         chrono::Utc::now().into(),
    //         21.0454.into(),
    //         50.into(),
    //     );
    //     let trade = order_book.execute(order).unwrap();
    //     assert_eq!(trade.order_id, crate::primitives::Oid::new(3));
    //     assert_eq!(trade.volume, 50.into());
    //     assert_eq!(trade.filled_volume, 0.into());
    //     assert_eq!(trade.executions.len(), 0);

    //     let order = &crate::Order::new_limit(
    //         crate::primitives::Oid::new(2),
    //         crate::OrderSide::Buy,
    //         chrono::Utc::now().into(),
    //         21.0455.into(),
    //         125.into(),
    //     );

    //     let trade = order_book.execute(order).unwrap();
    //     assert_eq!(trade.order_id, crate::primitives::Oid::new(2));
    //     assert_eq!(trade.volume, 125.into());
    //     assert_eq!(trade.filled_volume, 125.into());
    //     assert_eq!(trade.executions.len(), 2);
    //     let execution = &trade.executions[0];
    //     assert_eq!(execution.order_id, crate::primitives::Oid::new(1));
    //     assert_eq!(execution.price, 21.0453.into());
    //     assert_eq!(execution.volume, 100.into());
    //     let execution = &trade.executions[1];
    //     assert_eq!(execution.order_id, crate::primitives::Oid::new(3));
    //     assert_eq!(execution.price, 21.0454.into());
    //     assert_eq!(execution.volume, 25.into());
    // }

    // #[test]
    // fn test_market_order_should_result_in_empty_order_book() {
    //     let mut order_book = crate::OrderBook::default();
    //     let order = &crate::Order::new_limit(
    //         crate::primitives::Oid::new(1),
    //         crate::OrderSide::Sell,
    //         chrono::Utc::now().into(),
    //         21.0453.into(),
    //         100.into(),
    //     );
    //     let _ = order_book.execute(order);

    //     let order = &crate::Order::new_limit(
    //         crate::primitives::Oid::new(2),
    //         crate::OrderSide::Sell,
    //         chrono::Utc::now().into(),
    //         21.0454.into(),
    //         50.into(),
    //     );
    //     let _ = order_book.execute(order);

    //     let order = &crate::Order::new_market(
    //         crate::primitives::Oid::new(3),
    //         crate::OrderSide::Buy,
    //         chrono::Utc::now().into(),
    //         150.into(),
    //     );
    //     let trade = order_book.execute(order).unwrap();
    //     assert_eq!(trade.order_id, crate::primitives::Oid::new(3));
    //     assert_eq!(trade.volume, 150.into());
    //     assert_eq!(trade.filled_volume, 150.into());
    //     assert_eq!(trade.executions.len(), 2);
    //     let execution = &trade.executions[0];
    //     assert_eq!(execution.order_id, crate::primitives::Oid::new(1));
    //     assert_eq!(execution.price, 21.0453.into());
    //     assert_eq!(execution.volume, 100.into());
    //     let execution = &trade.executions[1];
    //     assert_eq!(execution.order_id, crate::primitives::Oid::new(2));
    //     assert_eq!(execution.price, 21.0454.into());
    //     assert_eq!(execution.volume, 50.into());

    //     assert_eq!(order_book.orders.len(), 0);
    // }

    // #[test]
    // fn test_sell_market_order_should_result_in_empty_order_book() {
    //     let mut order_book = crate::OrderBook::default();
    //     let order = &crate::Order::new_limit(
    //         crate::primitives::Oid::new(1),
    //         crate::OrderSide::Buy,
    //         chrono::Utc::now().into(),
    //         21.0453.into(),
    //         100.into(),
    //     );
    //     let _ = order_book.execute(order);

    //     let order = &crate::Order::new_limit(
    //         crate::primitives::Oid::new(2),
    //         crate::OrderSide::Buy,
    //         chrono::Utc::now().into(),
    //         21.0454.into(),
    //         50.into(),
    //     );
    //     let _ = order_book.execute(order);

    //     let order = &crate::Order::new_market(
    //         crate::primitives::Oid::new(3),
    //         crate::OrderSide::Sell,
    //         chrono::Utc::now().into(),
    //         150.into(),
    //     );
    //     let trade = order_book.execute(order).unwrap();

    //     assert_eq!(trade.order_id, crate::primitives::Oid::new(3));
    //     assert_eq!(trade.volume, 150.into());
    //     assert_eq!(trade.filled_volume, 150.into());
    //     assert_eq!(trade.executions.len(), 2);
    //     let execution = &trade.executions[0];
    //     assert_eq!(execution.order_id, crate::primitives::Oid::new(2));
    //     assert_eq!(execution.price, 21.0454.into());
    //     assert_eq!(execution.volume, 50.into());
    //     let execution = &trade.executions[1];
    //     assert_eq!(execution.order_id, crate::primitives::Oid::new(1));
    //     assert_eq!(execution.price, 21.0453.into());
    //     assert_eq!(execution.volume, 100.into());

    //     assert_eq!(order_book.orders.len(), 0);
    // }
}
