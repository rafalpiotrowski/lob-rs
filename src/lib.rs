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
pub mod utils;
use itertools::Itertools;
use stable_vec::StableVec;
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

pub use primitives::{Oid, OrderSide, OrderType, Price, Timestamp, Volume};

/// LevelIndex is an index to a Level in a stable vec
type LevelIndex = usize;

// stable vec of levels, once added level will not change its index
// it will be removed only when the level is empty
// so when looking up the index we will get None
type LevelVec = StableVec<Level>;

// map of Limit -> LevelIndex
// this will allow for O(1) lookup of Limit levels
// this will only grow, since each limit need to point to a stable index in the stable level vec
type LevelMap = HashMap<Price, LevelIndex>;

// map of Order ID -> LimitOrder that contains full order data 
type OrderMap = HashMap<Oid, LimitOrder>;

/// Order
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Order {
    id: Oid,
    pub side: OrderSide,
    kind: OrderType,
    pub price: Option<Price>,
    pub volume: Volume,
    timestamp: Timestamp,
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

/// Limit Order
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct LimitOrder {
    id: Oid,
    side: OrderSide,
    timestamp: Timestamp,
    price: Price,
    volume: Volume,
    filled_volume: Option<Volume>,
}

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

/// Limit level
/// represents Price level and list of orders in FIFO order
#[derive(Debug, Clone)]
pub struct Level {
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

/// Limits (i.e. Price): 21.0453 to orders at that price
#[derive(Debug, Clone)]
pub struct Limits {
    /// side of the book
    side: OrderSide,
    /// map of LimitIndex -> Level (i.e. queue of orders at that Limit level)
    /// this will allow for O(1) lookup of Limit levels
    /// when inserting an order at a specific Limit level
    levels: LevelVec,
    // stable vector of Limit levels
    // once inserted limit will not change its index
    // and hence we will be able to make O(1) lookup to access orders at that level
    level_map: LevelMap,
    // for bids is max for asks is min limit
    best: Option<LevelIndex>,
}

impl Limits {
    /// Create a new Limit map
    pub fn new(side: OrderSide) -> Self {
        Limits {
            side,
            level_map: HashMap::new(),
            levels: StableVec::new(),
            best: None,
        }
    }

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
                self.level_map.insert(*price, index);
            }
            Some(index) => {
                // add the order to the existing Limit level
                if let Some(level) = self.levels.get_mut(*index) {
                    level.add_order(order);
                }
            }
        }
    }

    /// cancell order
    /// since we postopne removal of cancelled orders when filling the new order
    /// all we need to do is to update the total level volume so it is in sync
    pub fn cancel_order(&mut self, order: &LimitOrder) {
        self.level_map.get(&order.price).map(|index| {
            if let Some(level) = self.levels.get_mut(*index) {
                level.cancell_order(order);
            }
        });
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
        self.filled_volume += execution.volume.into();
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

/// Spread
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Spread(f64);

/// Limit Order Book
/// Trades are made when highest bid Limit is greater than or equal to the lowest ask Limit (spread is crossed)
/// If order cannot be filled immediately, it is added to the book
#[derive(Debug)]
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
    /// Create a new order book
    pub fn new() -> Self {
        OrderBook {
            bids: Limits::new(OrderSide::Buy),
            asks: Limits::new(OrderSide::Sell),
            orders: HashMap::new(),
            spread: None,
        }
    }

    /// takes an order object and either fills it or places it in the limit
    /// book, prints trades that have taken place as a result of the order
    pub fn execute(&mut self, order: &Order) -> Result<Trade, PlaceOrderError> {
        // we start with implementing market order

        let trade = match order.side {
            OrderSide::Buy => self.execute_buy(order),
            OrderSide::Sell => self.execute_sell(order),
        }?;

        // update min/max limits
        self.update_best_limit();

        // update spread
        self.update_spreads();
        Ok(trade)
    }

    fn update_spreads(&mut self) {
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

    fn execute_buy(&mut self, order: &Order) -> Result<Trade, PlaceOrderError> {
        let trade = self.fill_buy_order(&order)?;
        match order.kind {
            OrderType::Market => {
                // we do not need to add the order to the book
            }
            OrderType::Limit => {
                if trade.filled_volume < order.volume {
                    // add the order to the book
                    let limit_order = LimitOrder::try_from(order).map_err(|_| {
                        PlaceOrderError::OrderCannotBePlaced("not an market order".to_string())
                    })?;
                    self.bids.add_order(&limit_order);
                    self.orders.insert(limit_order.id, limit_order);
                }
            }
        }
        Ok(trade)
    }

    fn update_best_limit(&mut self) {
        self.bids
            .levels
            .values()
            .filter(|l| l.total_volume > 0.into())
            .max()
            .map(|max| {
                self.bids.best = self.bids.level_map.get(&max.price).copied();
            });
        self.asks
            .levels
            .values()
            .filter(|l| l.total_volume > 0.into())
            .min()
            .map(|min| {
                self.asks.best = self.asks.level_map.get(&min.price).copied();
            });
    }

    fn execute_sell(&mut self, order: &Order) -> Result<Trade, PlaceOrderError> {
        let trade = self.fill_sell_order(order)?;
        match order.kind {
            OrderType::Market => {
                // we do not need to add the order to the book
            }
            OrderType::Limit => {
                if trade.filled_volume < order.volume {
                    // add the order to the book
                    let limit_order = LimitOrder::try_from(order).map_err(|_| {
                        PlaceOrderError::OrderCannotBePlaced("not an market order".to_string())
                    })?;
                    self.asks.add_order(&limit_order);
                    self.orders.insert(limit_order.id, limit_order);
                }
            }
        }
        Ok(trade)
    }

    fn fill_buy_order(&mut self, buy_order: &Order) -> Result<Trade, PlaceOrderError> {
        // find the lowest sell Limit
        // if the lowest sell Limit is less than or equal to the buy Limit, we can fill the order, substracting the volume
        // if the lowest sell Limit is greater than the buy Limit, we add the order to the book, with the volume
        // equal to the order quantity

        let mut buy_volume = buy_order.volume;
        let mut trade = Trade::new(buy_order.id, buy_order.volume);

        let sorted = self
            .asks
            .levels
            .values_mut()
            .filter(|l| filter_limit_for_buy(l, &buy_order.price))
            .sorted();

        'top: for l in sorted {
            loop {
                // peek order at front of the level
                if let Some(oid) = l.orders.front() {
                    // todo: remove might trigger memcpy
                    // although we need to get the owned value otherwise we will be borrowing self hence problem with borrow checker
                    let Some(mut sell_order) = self.orders.remove(&oid) else {
                        // if there is no order then it might have been cancelled
                        // and removed from the map, and since we pospone the removal of orders from the level
                        // till we encounter such order, we can safely remove the order from the level
                        l.orders.pop_front();
                        continue;
                    };
                    let sell_volume = sell_order.volume;
                    if sell_volume <= buy_volume {
                        // fill the sell order
                        trade.add_execution(Execution::new(
                            sell_order.id,
                            sell_order.price,
                            sell_volume.into(),
                        ));
                        // remove order from the level
                        l.orders.pop_front();
                        l.cancell_order(&sell_order);
                        sell_order.volume = Volume::ZERO;
                        buy_volume -= sell_volume;
                    } else {
                        // fill the buy order, put the order back to the book
                        let execution = Execution::new(sell_order.id, sell_order.price, buy_volume);
                        trade.add_execution(execution);
                        sell_order.volume -= buy_volume;
                        buy_volume = Volume::ZERO;
                    }
                    // we should put back the sell order if it was not completely filled
                    if !sell_order.volume.is_zero() {
                        self.orders.insert(sell_order.id, sell_order);
                    }
                    // if buy order was filled completely, we can break the loop
                    if buy_volume.is_zero() {
                        break 'top;
                    }
                    // otherwise we still have volume to fill
                } else {
                    // no more orders at the level, we can move to the next level
                    break;
                }
            }
        }
        Ok(trade)
    }

    fn fill_sell_order(&mut self, sell_order: &Order) -> Result<Trade, PlaceOrderError> {
        // find the highest bid Limit
        // if the highest bid Limit is greater than or equal to the ask Limit, we can fill the order, substracting the volume
        // if the highest bid Limit is less than the ask Limit, we add the order to the book, with the volume
        // equal to the order quantity

        let mut sell_volume = sell_order.volume;
        let mut trade = Trade::new(sell_order.id, sell_order.volume);

        let sorted = self
            .bids
            .levels
            .values_mut()
            .filter(|l| filter_limit_for_sell(l, &sell_order.price))
            .sorted_by(sort_limit_descending);

        'top: for l in sorted {
            loop {
                // peek order at front of the level
                if let Some(oid) = l.orders.front() {
                    // todo: remove might trigger memcpy
                    // although we need to get the owned value otherwise we will be borrowing self hence problem with borrow checker
                    let Some(mut buy_order) = self.orders.remove(&oid) else {
                        // if there is no order then it might have been cancelled
                        // and removed from the map, and since we pospone the removal of orders from the level
                        // till we encounter such order, we can safely remove the order from the level
                        l.orders.pop_front();
                        continue;
                    };
                    let buy_volume = buy_order.volume;
                    if buy_volume <= sell_volume {
                        // fill the sell order
                        trade.add_execution(Execution::new(
                            buy_order.id,
                            buy_order.price,
                            buy_volume.into(),
                        ));
                        // remove order from the level
                        l.orders.pop_front();
                        l.cancell_order(&buy_order);
                        buy_order.volume = Volume::ZERO;
                        sell_volume -= buy_volume;
                    } else {
                        // fill the buy order, put the order back to the book
                        let execution = Execution::new(buy_order.id, buy_order.price, sell_volume);
                        trade.add_execution(execution);
                        buy_order.volume -= sell_volume;
                        sell_volume = Volume::ZERO;
                    }
                    // we should put back the sell order if it was not completely filled
                    if !buy_order.volume.is_zero() {
                        self.orders.insert(buy_order.id, buy_order);
                    }
                    // if sell order was filled completely, we can break the loop
                    if sell_volume.is_zero() {
                        break 'top;
                    }
                    // otherwise we still have volume to fill
                } else {
                    // no more orders at the level, we can move to the next level
                    break;
                }
            }
        }
        Ok(trade)
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
            .map(|index| limit_map.levels[*index].total_volume)
    }
}

// we want to inline since this is a small function and we want to avoid the overhead of a function call
#[inline]
fn sort_limit_descending<'a, 'b>(l: &'a &mut Level, r: &'b &mut Level) -> std::cmp::Ordering {
    l.price.cmp(&r.price).reverse()
}
#[inline]
fn filter_limit_for_buy<'a>(l: &'a &mut Level, price: &Option<Price>) -> bool {
    if l.total_volume > 0.into() {
        // in case price is none, we want to return true since we are in market order which has no price
        return price.map(|p| l.price <= p).unwrap_or(true);
    }
    false
}
#[inline]
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
        let mut limit_map = crate::Limits::new(crate::OrderSide::Buy);
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
        let order_book = crate::OrderBook::new();
        assert_eq!(order_book.bids.side, crate::OrderSide::Buy);
        assert_eq!(order_book.asks.side, crate::OrderSide::Sell);
        assert_eq!(order_book.orders.len(), 0);
        assert_eq!(order_book.spread, None);
    }

    #[test]
    fn test_cancel_order() {
        let mut order_book = crate::OrderBook::new();
        let order = &crate::Order::new_limit(
            crate::primitives::Oid::new(1),
            crate::OrderSide::Buy,
            chrono::Utc::now().into(),
            21.0453.into(),
            100.into(),
        );
        let _ = order_book.execute(order);
        assert_eq!(order_book.orders.len(), 1);
        let order = order_book.cancel_order(crate::primitives::Oid::new(1)).unwrap();
        assert_eq!(order_book.orders.len(), 0);
        assert_eq!(order.order_id, crate::primitives::Oid::new(1));
        assert_eq!(order.status, crate::CancellationStatus::Cancelled);

        let order = &crate::Order::new_limit(
            crate::primitives::Oid::new(2),
            crate::OrderSide::Buy,
            chrono::Utc::now().into(),
            21.0453.into(),
            50.into(),
        );
        let _ = order_book.execute(order);
        assert_eq!(order_book.orders.len(), 1);
        let order = order_book.cancel_order(crate::primitives::Oid::new(2)).unwrap();
        assert_eq!(order_book.orders.len(), 0);
        assert_eq!(order.order_id, crate::primitives::Oid::new(2));
        assert_eq!(order.status, crate::CancellationStatus::Cancelled);
    }

    #[test]
    fn test_execute_buy_order() {
        let mut order_book = crate::OrderBook::new();
        let order = &crate::Order::new_limit(
            crate::primitives::Oid::new(1),
            crate::OrderSide::Sell,
            chrono::Utc::now().into(),
            21.0453.into(),
            100.into(),
        );
        let trade = order_book.execute(order).unwrap();
        assert_eq!(trade.order_id, crate::primitives::Oid::new(1));
        assert_eq!(trade.volume, 100.into());
        assert_eq!(trade.filled_volume, 0.into());
        assert_eq!(trade.executions.len(), 0);

        let order = &crate::Order::new_limit(
            crate::primitives::Oid::new(3),
            crate::OrderSide::Sell,
            chrono::Utc::now().into(),
            21.0454.into(),
            50.into(),
        );
        let trade = order_book.execute(order).unwrap();
        assert_eq!(trade.order_id, crate::primitives::Oid::new(3));
        assert_eq!(trade.volume, 50.into());
        assert_eq!(trade.filled_volume, 0.into());
        assert_eq!(trade.executions.len(), 0);

        let order = &crate::Order::new_limit(
            crate::primitives::Oid::new(2),
            crate::OrderSide::Buy,
            chrono::Utc::now().into(),
            21.0455.into(),
            125.into(),
        );

        let trade = order_book.execute(order).unwrap();
        assert_eq!(trade.order_id, crate::primitives::Oid::new(2));
        assert_eq!(trade.volume, 125.into());
        assert_eq!(trade.filled_volume, 125.into());
        assert_eq!(trade.executions.len(), 2);
        let execution = &trade.executions[0];
        assert_eq!(execution.order_id, crate::primitives::Oid::new(1));
        assert_eq!(execution.price, 21.0453.into());
        assert_eq!(execution.volume, 100.into());
        let execution = &trade.executions[1];
        assert_eq!(execution.order_id, crate::primitives::Oid::new(3));
        assert_eq!(execution.price, 21.0454.into());
        assert_eq!(execution.volume, 25.into());
    }

    #[test]
    fn test_market_order_should_result_in_empty_order_book() {
        let mut order_book = crate::OrderBook::new();
        let order = &crate::Order::new_limit(
            crate::primitives::Oid::new(1),
            crate::OrderSide::Sell,
            chrono::Utc::now().into(),
            21.0453.into(),
            100.into(),
        );
        let _ = order_book.execute(order);

        let order = &crate::Order::new_limit(
            crate::primitives::Oid::new(2),
            crate::OrderSide::Sell,
            chrono::Utc::now().into(),
            21.0454.into(),
            50.into(),
        );
        let _ = order_book.execute(order);

        let order = &crate::Order::new_market(
            crate::primitives::Oid::new(3),
            crate::OrderSide::Buy,
            chrono::Utc::now().into(),
            150.into(),
        );
        let trade = order_book.execute(order).unwrap();
        assert_eq!(trade.order_id, crate::primitives::Oid::new(3));
        assert_eq!(trade.volume, 150.into());
        assert_eq!(trade.filled_volume, 150.into());
        assert_eq!(trade.executions.len(), 2);
        let execution = &trade.executions[0];
        assert_eq!(execution.order_id, crate::primitives::Oid::new(1));
        assert_eq!(execution.price, 21.0453.into());
        assert_eq!(execution.volume, 100.into());
        let execution = &trade.executions[1];
        assert_eq!(execution.order_id, crate::primitives::Oid::new(2));
        assert_eq!(execution.price, 21.0454.into());
        assert_eq!(execution.volume, 50.into());

        assert_eq!(order_book.orders.len(), 0);
    }

    #[test]
    fn test_sell_market_order_should_result_in_empty_order_book() {
        let mut order_book = crate::OrderBook::new();
        let order = &crate::Order::new_limit(
            crate::primitives::Oid::new(1),
            crate::OrderSide::Buy,
            chrono::Utc::now().into(),
            21.0453.into(),
            100.into(),
        );
        let _ = order_book.execute(order);

        let order = &crate::Order::new_limit(
            crate::primitives::Oid::new(2),
            crate::OrderSide::Buy,
            chrono::Utc::now().into(),
            21.0454.into(),
            50.into(),
        );
        let _ = order_book.execute(order);

        let order = &crate::Order::new_market(
            crate::primitives::Oid::new(3),
            crate::OrderSide::Sell,
            chrono::Utc::now().into(),
            150.into(),
        );
        let trade = order_book.execute(order).unwrap();

        assert_eq!(trade.order_id, crate::primitives::Oid::new(3));
        assert_eq!(trade.volume, 150.into());
        assert_eq!(trade.filled_volume, 150.into());
        assert_eq!(trade.executions.len(), 2);
        let execution = &trade.executions[0];
        assert_eq!(execution.order_id, crate::primitives::Oid::new(2));
        assert_eq!(execution.price, 21.0454.into());
        assert_eq!(execution.volume, 50.into());
        let execution = &trade.executions[1];
        assert_eq!(execution.order_id, crate::primitives::Oid::new(1));
        assert_eq!(execution.price, 21.0453.into());
        assert_eq!(execution.volume, 100.into());

        assert_eq!(order_book.orders.len(), 0);
    }
}