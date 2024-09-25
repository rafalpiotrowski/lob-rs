use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lob::{Order, OrderBook, OrderSide, Volume};

// create num_orders orders
// buy orders will have even ids, sell orders will have odd ids
// buy orders will have prices in descending order
// sell orders will have prices in ascending order
// when iterating through the first buy and sell orders we will construct order book that will have num_orders (50%/50%) buy/sell orders
fn setup_orders(num_orders: u64) -> Vec<Order> {
    let mut orders = Vec::with_capacity(num_orders as usize);

    // create stable list of orders that will build order book with 50%/50% buy/sell orders

    let mut buy_price_diff = 0.0;
    let mut buy_volume_count = 0;
    let mut sell_price_diff = 0.0;
    let mut sell_volume_count = 0;
    for i in 0..num_orders {
        let order = if i % 2 == 0 {
            if buy_price_diff > 98.0 {
                buy_price_diff = 0.0;
            }
            buy_price_diff += 1.0;
            if buy_volume_count == 100 {
                buy_volume_count = 0;
            }
            buy_volume_count += 1;
            Order::new_limit(
                black_box(i.into()),
                black_box(OrderSide::Buy),
                black_box(chrono::Utc::now().into()),
                black_box(100.0 - buy_price_diff).into(),
                black_box(100 + buy_volume_count).into(),
            )
        } else {
            if sell_price_diff > 98.0 {
                sell_price_diff = 0.0;
            }
            sell_price_diff += 1.0;
            if sell_volume_count == 100 {
                sell_volume_count = 0;
            }
            sell_volume_count += 1;
            Order::new_limit(
                black_box(i.into()),
                black_box(OrderSide::Sell),
                black_box(chrono::Utc::now().into()),
                black_box(100.0 + sell_price_diff).into(),
                black_box(100 + sell_volume_count).into(),
            )
        };
        orders.push(order);
    }

    // create orders that will be matched with the stable list of orders
    // and the result should be empty order book

    let mut id = num_orders + 1;

    let buy_volume = orders
        .iter()
        .filter(|o| o.side == OrderSide::Buy)
        .map(|o| o.volume)
        .sum::<Volume>();
    let sell_volume = orders
        .iter()
        .filter(|o| o.side == OrderSide::Sell)
        .map(|o| o.volume)
        .sum::<Volume>();

    // add sell market order that will be matched with all buy orders
    orders.push(Order::new_market(
        black_box(id.into()),
        black_box(OrderSide::Sell),
        black_box(chrono::Utc::now().into()),
        black_box(buy_volume),
    ));

    id += 1;

    // add buy market order that will be matched with all sell orders
    orders.push(Order::new_market(
        black_box(id.into()),
        black_box(OrderSide::Buy),
        black_box(chrono::Utc::now().into()),
        black_box(sell_volume),
    ));

    orders
}

fn bench_order_matching(c: &mut Criterion) {
    let orders = setup_orders(10000);
    c.bench_function("order_matching", |b| {
        b.iter(|| {
            let mut order_book = OrderBook::default();
            for order in orders.iter() {
                let _ = order_book.execute(order);
            }
        })
    });
}

criterion_group!(benches, bench_order_matching);
criterion_main!(benches);
