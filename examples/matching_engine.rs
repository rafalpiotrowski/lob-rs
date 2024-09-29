use std::collections::VecDeque;

use lob::{LimitOrder, Order, OrderBook, OrderSide, OrderType, PlaceOrderError, Price, Trade};

pub fn main() {
    let mut order_book = OrderBook::default();
}

pub trait Matching {
    fn match_orders(&mut self, order_book: &mut OrderBook) -> Vec<Trade>;
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

impl Exchange {
    pub fn new() -> Self {
        Self {
            matching_engine: MatchingEngine::new(OrderBook::default()),
        }
    }
    pub fn execute_order(&mut self, order: Order) -> Result<Vec<Trade>, PlaceOrderError> {
        self.matching_engine.place_order(order)?;
        self.matching_engine.match_orders()
    }
}

impl MatchingEngine {
    pub fn new(order_book: OrderBook) -> Self {
        Self {
            order_book,
            min_price: Price(f64::MIN),
            max_price: Price(f64::MAX),
            market_orders: VecDeque::new(),
        }
    }

    pub fn place_order(&mut self, order: Order) -> Result<(), PlaceOrderError> {
        // this is the entry point to matching engine
        // if exchange for example
        if order.kind == OrderType::Limit {
            if order.price.is_none() {
                return Err(PlaceOrderError::OrderCannotBePlaced(
                    "price is required for limit order".to_string(),
                ));
            }
            if order.price.unwrap() < self.min_price {
                return Err(PlaceOrderError::OrderCannotBePlaced(
                    "price is too low".to_string(),
                ));
            }
            if order.price.unwrap() > self.max_price {
                return Err(PlaceOrderError::OrderCannotBePlaced(
                    "price is too high".to_string(),
                ));
            }
            self.order_book
                .add_order(LimitOrder::try_from(&order).unwrap());
        } else {
            // market order
            self.market_orders.push_back(order);
        }

        Ok(())
    }

    pub fn match_orders(&mut self) -> Result<Vec<Trade>, PlaceOrderError> {
        let mut trades = Vec::new();
        while let Some(order) = self.market_orders.pop_front() {
            let trade = Trade::new(order.id, order.volume);
            let trade = match order.side {
                OrderSide::Buy => self.order_book.fill_buy_order(trade, order.price)?,
                OrderSide::Sell => self.order_book.fill_sell_order(trade, order.price)?,
            };
            trades.push(trade);
        }

        let best_buy = self.order_book.get_best_buy();
        let best_sell = self.order_book.get_best_sell();
        if best_sell.is_some() && best_buy.is_some() {
            if best_sell.unwrap() <= best_buy.unwrap() {}
        }
        Ok(trades)
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
    fn match_orders(&mut self, order_book: &mut OrderBook) -> Vec<Trade> {
        todo!("Implement matching engine")
    }
}
