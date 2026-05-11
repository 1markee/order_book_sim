use std::collections::{BTreeMap, HashMap};

use crate::events::BookEvent;
use crate::level::PriceLevel;
use crate::order::{Order, OrderId, OrderType, Price, Side};

#[derive(Debug, Default)]
pub struct OrderBook {
    pub bids: BTreeMap<Price, PriceLevel>,
    pub asks: BTreeMap<Price, PriceLevel>,
    pub index: HashMap<OrderId, (Side, Price)>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    pub fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    pub fn submit(&mut self, order: Order) -> Vec<BookEvent> {
        let mut events = Vec::new();

        if order.qty == 0 {
            events.push(BookEvent::Rejected {
                id: order.id,
                reason: "zero quantity",
            });
            return events;
        }
        if matches!(order.kind, OrderType::Limit) && order.price.is_none() {
            events.push(BookEvent::Rejected {
                id: order.id,
                reason: "limit without price",
            });
            return events;
        }

        events.push(BookEvent::Accepted { id: order.id });

        let remaining = match order.side {
            Side::Bid => self.match_buy(order.id, order.kind, order.price, order.qty, &mut events),
            Side::Ask => self.match_sell(order.id, order.kind, order.price, order.qty, &mut events),
        };

        if remaining > 0 && matches!(order.kind, OrderType::Limit) {
            let price = order.price.expect("limit price validated above");
            let levels = match order.side {
                Side::Bid => &mut self.bids,
                Side::Ask => &mut self.asks,
            };
            let level = levels.entry(price).or_default();
            level.orders.push_back(Order {
                qty: remaining,
                ..order
            });
            level.total_qty += remaining;
            self.index.insert(order.id, (order.side, price));
        }

        events
    }

    pub fn cancel(&mut self, id: OrderId) -> Option<BookEvent> {
        let (side, price) = self.index.remove(&id)?;
        let levels = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };
        let level = levels.get_mut(&price)?;
        let pos = level.orders.iter().position(|o| o.id == id)?;
        let removed = level.orders.remove(pos)?;
        level.total_qty -= removed.qty;
        if level.orders.is_empty() {
            levels.remove(&price);
        }
        Some(BookEvent::Canceled { id })
    }

    fn match_buy(
        &mut self,
        taker_id: OrderId,
        taker_kind: OrderType,
        taker_price: Option<Price>,
        mut remaining: u64,
        events: &mut Vec<BookEvent>,
    ) -> u64 {
        while remaining > 0 {
            let best_price = match self.asks.iter().next() {
                Some((&p, _)) => p,
                None => break,
            };
            if matches!(taker_kind, OrderType::Limit)
                && taker_price.expect("limit price validated") < best_price
            {
                break;
            }

            let level = self.asks.get_mut(&best_price).expect("level exists");
            let mut emptied = false;

            while remaining > 0 {
                let (front_id, front_qty) = match level.orders.front() {
                    Some(o) => (o.id, o.qty),
                    None => {
                        emptied = true;
                        break;
                    }
                };
                let trade_qty = remaining.min(front_qty);

                events.push(BookEvent::Trade {
                    maker: front_id,
                    taker: taker_id,
                    price: best_price,
                    qty: trade_qty,
                });

                level.total_qty -= trade_qty;
                remaining -= trade_qty;

                if front_qty == trade_qty {
                    level.orders.pop_front();
                    self.index.remove(&front_id);
                    if level.orders.is_empty() {
                        emptied = true;
                        break;
                    }
                } else {
                    level.orders.front_mut().expect("non-empty").qty -= trade_qty;
                }
            }

            if emptied {
                self.asks.remove(&best_price);
            }
        }
        remaining
    }

    fn match_sell(
        &mut self,
        taker_id: OrderId,
        taker_kind: OrderType,
        taker_price: Option<Price>,
        mut remaining: u64,
        events: &mut Vec<BookEvent>,
    ) -> u64 {
        while remaining > 0 {
            let best_price = match self.bids.iter().next_back() {
                Some((&p, _)) => p,
                None => break,
            };
            if matches!(taker_kind, OrderType::Limit)
                && taker_price.expect("limit price validated") > best_price
            {
                break;
            }

            let level = self.bids.get_mut(&best_price).expect("level exists");
            let mut emptied = false;

            while remaining > 0 {
                let (front_id, front_qty) = match level.orders.front() {
                    Some(o) => (o.id, o.qty),
                    None => {
                        emptied = true;
                        break;
                    }
                };
                let trade_qty = remaining.min(front_qty);

                events.push(BookEvent::Trade {
                    maker: front_id,
                    taker: taker_id,
                    price: best_price,
                    qty: trade_qty,
                });

                level.total_qty -= trade_qty;
                remaining -= trade_qty;

                if front_qty == trade_qty {
                    level.orders.pop_front();
                    self.index.remove(&front_id);
                    if level.orders.is_empty() {
                        emptied = true;
                        break;
                    }
                } else {
                    level.orders.front_mut().expect("non-empty").qty -= trade_qty;
                }
            }

            if emptied {
                self.bids.remove(&best_price);
            }
        }
        remaining
    }
}
