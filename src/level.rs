use std::collections::VecDeque;

use crate::order::Order;

#[derive(Debug, Default)]
pub struct PriceLevel {
    pub orders: VecDeque<Order>,
    pub total_qty: u64,
}

impl PriceLevel {
    pub fn new() -> Self {
        Self::default()
    }
}
