pub type OrderId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: OrderId,
    pub side: Side,
    pub kind: OrderType,
    pub price: Option<Price>,
    pub qty: u64,
    pub ts: u64,
}
