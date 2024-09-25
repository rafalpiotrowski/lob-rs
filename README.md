[![Crates.io](https://img.shields.io/crates/v/lob)](https://crates.io/crates/lob)
[![minimum rustc version](https://img.shields.io/badge/rustc-1.64+-green.svg)](https://blog.rust-lang.org/2019/11/07/Rust-1.64.0.html)
[![Build Status](https://github.com/rafalpiotrowski/lob-rs/actions/workflows/main.yml/badge.svg?branch=main)](https://github.com/rafalpiotrowski/lob-rs)

# Limit Order Book

## Orders Types

### Market

This order must be executed immediately at the current market price. If there is no necessary amount, then the order is executed as soon as possible.

### MOO - Market On Open

A market order of such type can be executed only at the opening of the trading day. Its execution is guaranteed provided there is liquidity.

### MOC - Market On Close

These orders are similar to MOO, but the market order can be executed at the closing of the trading day only. The execution is guaranteed provided there is liquidity.

### Pending orders

A request enables to control the limit prices for buying and selling. Sometimes such orders can not be executed straightaway or at all. The price is always better for limit orders.

#### Limit orders

##### LOD - Limit On Open

Like MOO, LOO can be executed at the opening of the trading day only. However, limit orders of this type enable for setting instructions for the price. The execution is not guaranteed.

##### LOC - Limit On Close

These orders are similar to LOO, but the market order can be executed at the closing of the trading day only. The execution is not guaranteed.

### Conditional orders

These are all requests except limit ones that require extra conditions to be activated and executed.

When we want to tailor our orders for some trading strategy, we can do it with the help of conditional orders – the ones which are activated only after the condition has been satisfied. Thus, their main aim is to minimize the risk of significant financial losses.

#### MIT - Market if Touched

#### ATO - At the opening

#### Peg orders

This is a kind of limit orders. It changes its price automatically after the change of bid or ask prices. In price instructions a shift for the worse from the current best bid/ask price is indicated.

##### OSO - One Sends Other

This order consists of the main order and a group of linked orders. Orders can work with different instruments. If the main order is executed,all linked orders are then sent.

Now when we’ve briefly reviewed orders theory, let’s get down to practice.

##### OCO - One Cancel Other

This is a pair of orders. It usually consists of a stop order and a limit order. Both orders work with the same instrument. If one of them is executed, the second is automatically cancelled.
A simple example: let’s imagine that we’ve bought an asset for ＄10. We aim at getting at least ＄3 of profit and limiting the losses to ＄2 at most. Then our stop price will be ＄8 and our limit price ＄13.

#### Stop orders

Stop order is a bid or an ask for a certain amount of a financial asset at a specified price or worse. As soon as the price equals or exceeds the specified stop price for a bid order, it automatically turns into a market order and is executed on general grounds. It works the same way with a stop order for selling. As soon as the price reaches the level set in the order and continues to fall, the order turns into a market one. This way, execution is guaranteed for active stop orders. Stop orders let us minimize potential losses, that’s why stop loss is the synonym for the stop order.

##### Stop Loss

##### Stop Limit

Unlike usual stop orders, where just the stop price is specified, stop-limit orders require also a limit price.
As soon as the asset price is equal or worse than the stop price, a limit order with the specified limit price is automatically created. The execution of such order is not guaranteed.

##### Trailing Stop

Unlike usual stop orders, where the stop price is shown in absolute units, trailing orders use percentage. After activation, it turns into a usual market order. This way, the stop price is linked to the market price and grows with it. For example, we’ve placed an order for 10 dollars and set the stop price at 10% from the current one, i.g. the absolute stop price equals 9 dollars. Then, the asset grew to 15 dollars alongside with the stop price which grew to 13.5 dollars.
This order type is used to minimize losses as well as maximize profit.

##### Trailing Stop Limit

Unlike trailing stop orders, this order turns into a limit one, not a market one, after activation.

### Time In Force

Indicate how long an order will remain active before it is executed or expires.

### DAY

These orders expire at the end of the trading day.

### GTC (Good till canceled)

This order is valid until it is cancelled. To make sure they don’t stay at the market indefinitely, a special limitation is imposed, which is usually from 30 to 90 days.

### FOK (Fill Or Kill)

The order must be fully filled or immediately cancelled.

### IOC (Immediate Or Cancel)

The order is immediately executed or cancelled by the exchange. Unlike FOK, this type allows for partial fulfillment.

### GTD (Good-til-Date/Time)

### GAT (Good-after-Time/Date)
