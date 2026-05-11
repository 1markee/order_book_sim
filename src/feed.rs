pub mod random {
    use crate::order::{Order, OrderId, OrderType, Price, Side};

    pub struct RandomFeed {
        state: u64,
        next_id: OrderId,
        mid: i64,
        ts: u64,
    }

    impl RandomFeed {
        pub fn new(seed: u64) -> Self {
            Self {
                state: seed.max(1),
                next_id: 1,
                mid: 10_000,
                ts: 0,
            }
        }

        fn next_u64(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
            x
        }
    }

    impl Iterator for RandomFeed {
        type Item = Order;

        fn next(&mut self) -> Option<Order> {
            let r = self.next_u64();
            let side = if r & 1 == 0 { Side::Bid } else { Side::Ask };
            let offset = ((r >> 1) % 21) as i64 - 10;
            let price = Price(self.mid + offset);
            let qty = ((r >> 6) % 10) + 1;
            let id = self.next_id;
            self.next_id += 1;
            self.ts += 1;
            Some(Order {
                id,
                side,
                kind: OrderType::Limit,
                price: Some(price),
                qty,
                ts: self.ts,
            })
        }
    }
}

pub mod lobster {
    use std::path::Path;

    use serde::Deserialize;

    use crate::order::{Order, OrderId, OrderType, Price, Side};

    #[derive(Debug, Clone, Copy, Deserialize)]
    pub struct MessageRow {
        pub time: f64,
        pub event_type: u8,
        pub order_id: i64,
        pub size: u64,
        pub price: i64,
        pub direction: i8,
    }

    #[derive(Debug, Clone)]
    pub enum LobsterEvent {
        Submit(Order),
        Cancel { id: OrderId, qty: u64 },
        Delete { id: OrderId },
        Execute { id: OrderId, qty: u64 },
        Other,
    }

    pub fn read_messages<P, F>(path: P, mut sink: F) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        F: FnMut(LobsterEvent),
    {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(path)?;
        let mut ts: u64 = 0;
        for result in rdr.deserialize::<MessageRow>() {
            ts += 1;
            let row = result
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            sink(convert(row, ts));
        }
        Ok(())
    }

    fn convert(row: MessageRow, ts: u64) -> LobsterEvent {
        let id = row.order_id as OrderId;
        let side = match row.direction {
            1 => Side::Bid,
            -1 => Side::Ask,
            _ => return LobsterEvent::Other,
        };
        match row.event_type {
            1 => LobsterEvent::Submit(Order {
                id,
                side,
                kind: OrderType::Limit,
                price: Some(Price(row.price)),
                qty: row.size,
                ts,
            }),
            2 => LobsterEvent::Cancel { id, qty: row.size },
            3 => LobsterEvent::Delete { id },
            4 => LobsterEvent::Execute { id, qty: row.size },
            _ => LobsterEvent::Other,
        }
    }
}
