use crate::order::{OrderId, Price};

#[derive(Debug, Clone)]
pub enum BookEvent {
    Accepted {
        id: OrderId,
    },
    Rejected {
        id: OrderId,
        reason: &'static str,
    },
    Canceled {
        id: OrderId,
    },
    Trade {
        maker: OrderId,
        taker: OrderId,
        price: Price,
        qty: u64,
    },
}
