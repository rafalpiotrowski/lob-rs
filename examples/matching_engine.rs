use std::collections::VecDeque;
use thiserror::Error;

use lob::{LimitOrder, Oid, Order, OrderBook, OrderBookError, OrderSide, OrderType, Price, Volume};

pub fn main() {
    let _ = Exchange::new();
}

pub trait Matching {
    fn match_orders(&mut self) -> Vec<Trade>;
}

pub struct MatchingEngine {
    order_book: OrderBook,
    min_price: Price,
    max_price: Price,
    // queue of market orders, that should be matched first in first out
    market_orders: VecDeque<Order>,
}

pub struct Exchange {
    matching_engine: MatchingEngine,
}

#[derive(Error, Debug)]
pub enum ExchangeError {
    #[error("Failed to match error: {0}")]
    MatchingError(#[from] MatchingEngineError),
}

#[derive(Error, Debug)]
pub enum MatchingEngineError {
    #[error("OrderBook error: {0}")]
    OrderBookError(#[from] OrderBookError),
    #[error("Order price is too low")]
    OrderPriceTooLowError(),
    #[error("Order price is too high")]
    OrderPriceTooHighError(),
    #[error("Limit Order price is required")]
    MissingPriceError(),
    #[error("No market orders to match")]
    NoMarketOrdersError(),
    #[error("No orders to match")]
    NoOrdersToMatchError(),
}

impl Exchange {
    pub fn new() -> Self {
        Self {
            matching_engine: MatchingEngine::new(),
        }
    }
    pub fn place_order_single(&mut self, order: Order) -> Result<(), ExchangeError> {
        // place a single order in a proper matching engine for later matching
        self.matching_engine.place_order(order)?;
        Ok(())
    }
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            order_book: OrderBook::default(),
            min_price: Price(f64::MIN),
            max_price: Price(f64::MAX),
            market_orders: VecDeque::new(),
        }
    }

    pub fn had_market_orders(&self) -> bool {
        !self.market_orders.is_empty()
    }

    pub fn place_order(&mut self, order: Order) -> Result<(), MatchingEngineError> {
        // this is the entry point to matching engine
        // if exchange for example
        if order.kind == OrderType::Limit {
            if order.price.is_none() {
                return Err(MatchingEngineError::MissingPriceError());
            }
            if order.price.unwrap() < self.min_price {
                return Err(MatchingEngineError::OrderPriceTooLowError());
            }
            if order.price.unwrap() > self.max_price {
                return Err(MatchingEngineError::OrderPriceTooHighError());
            }
            self.order_book
                .add_order(LimitOrder::try_from(&order).unwrap());
        } else {
            // market order
            self.market_orders.push_back(order);
        }

        Ok(())
    }

    // pub fn pop_and_match_first_market_order(&mut self) -> Result<Trade, MatchingEngineError> {
    //     let Some(order) = self.market_orders.pop_front() else {
    //         return Err(MatchingEngineError::NoMarketOrdersError());
    //     };

    //     let trade = Trade::new(order.id, order.volume);
    //     let trade = match order.side {
    //         OrderSide::Buy => self.order_book.fill_buy_order(trade, order.price),
    //         OrderSide::Sell => self.order_book.fill_sell_order(trade, order.price),
    //     }?;
    //     Ok(trade)
    // }

    pub fn can_match_orders(&self) -> bool {
        let best_buy = self.order_book.get_best_buy();
        let best_sell = self.order_book.get_best_sell();
        best_sell.is_some() && best_buy.is_some()
    }

    pub fn match_orders(&mut self) -> Result<(), MatchingEngineError> {
        if !self.can_match_orders() {
            return Err(MatchingEngineError::NoOrdersToMatchError());
        }
        self.order_book.match_orders();
        Ok(())
    }

    // fn match_buy_side(&mut self) -> Result<Trade, PlaceOrderError> {
    //     let trade = self.order_book.fill_buy_order(order)?;
    //     match order.kind {
    //         OrderType::Market => {
    //             // we do not need to add the order to the book
    //         }
    //         OrderType::Limit => {
    //             if trade.filled_volume < order.volume {
    //                 // add the order to the book
    //                 let limit_order = LimitOrder::try_from(order).map_err(|_| {
    //                     PlaceOrderError::OrderCannotBePlaced("not an market order".to_string())
    //                 })?;
    //                 self.bids.add_order(&limit_order);
    //                 self.orders.insert(limit_order.id, limit_order);
    //             }
    //         }
    //     }
    //     Ok(trade)
    // }
}

impl Matching for MatchingEngine {
    fn match_orders(&mut self) -> Vec<Trade> {
        todo!("Implement matching engine")
    }
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
