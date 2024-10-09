use glommio::prelude::*;
use std::collections::VecDeque;
use thiserror::Error;

use clap::{command, Parser};
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, LazyLock};
use tracing_subscriber::EnvFilter;

use lob::{Fill, LimitOrder, Oid, Order, OrderBook, OrderBookError, OrderType, Price, Volume};

static RUNNING: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::from(true));

fn sig_int_handler() {
    RUNNING.store(false, Ordering::SeqCst);
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    cpu_id: Option<usize>,
}

pub fn main() -> std::io::Result<()> {
    println!("Welcome to the exchange! Gateway to MatchingEngine!");

    tracing_subscriber::fmt()
        .pretty()
        .with_thread_names(true)
        // enable everything
        .with_env_filter(EnvFilter::from_default_env())
        // sets this to be the default, global collector for this application.
        .init();

    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        sig_int_handler();
    })
    .expect("Error setting Ctrl-C handler");

    let args = Args::parse();

    let cpu_placement = args.cpu_id.map_or(Placement::Unbound, Placement::Fixed);

    let builder = LocalExecutorBuilder::new(cpu_placement.clone()).name("matching-engine");
    let handle = builder.spawn(|| async move {
        tracing::info!("Done!");
    })?;

    tracing::info!("MatchingEngine running on CPU {:?}", cpu_placement);

    handle.join().unwrap();

    tracing::info!("Goodbye!");

    Ok(())
}

pub trait Matching {
    fn match_orders(&mut self) -> Vec<Trade>;
}

#[derive(Debug, Default)]
pub struct MatchingEngine {
    order_book: OrderBook,
    min_price: Price,
    max_price: Price,
    // queue of market orders, that should be matched first in first out
    market_orders: VecDeque<Order>,
}

#[derive(Debug, Default)]
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
    pub fn initialize(&mut self) {
        self.matching_engine.set_min_price(Price::MIN);
        self.matching_engine.set_max_price(Price::MAX);
    }

    pub fn place_order_single(&mut self, order: Order) -> Result<(), ExchangeError> {
        // place a single order in a proper matching engine for later matching
        self.matching_engine.place_order(order)?;

        Ok(())
    }
}

impl MatchingEngine {
    pub fn set_min_price(&mut self, price: Price) {
        self.min_price = price;
    }

    pub fn set_max_price(&mut self, price: Price) {
        self.max_price = price;
    }

    pub fn has_market_orders(&self) -> bool {
        !self.market_orders.is_empty()
    }

    pub fn place_order(&mut self, order: Order) -> Result<(), MatchingEngineError> {
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

    pub fn can_match_orders(&self) -> bool {
        let best_buy = self.order_book.get_best_buy();
        let best_sell = self.order_book.get_best_sell();
        match (best_buy, best_sell) {
            (Some(buy_price), Some(sell_price)) => buy_price >= sell_price,
            _ => false,
        }
    }

    pub fn match_orders(&mut self) -> Result<Fill, MatchingEngineError> {
        self.order_book
            .find_and_fill_best_orders()
            .map_err(|e| e.into())
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
