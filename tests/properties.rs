use cc_order_book_sim::book::OrderBook;
use cc_order_book_sim::events::BookEvent;
use cc_order_book_sim::feed::random::RandomFeed;
use proptest::prelude::*;

proptest! {
    #[test]
    fn best_bid_strictly_below_best_ask(seed: u64, n in 1usize..500) {
        let mut book = OrderBook::new();
        for order in RandomFeed::new(seed).take(n) {
            book.submit(order);
            if let (Some(b), Some(a)) = (book.best_bid(), book.best_ask()) {
                prop_assert!(b < a, "crossed book: bid={:?} ask={:?}", b, a);
            }
        }
    }

    #[test]
    fn quantity_is_conserved(seed: u64, n in 1usize..500) {
        let mut book = OrderBook::new();
        let mut accepted_qty: u64 = 0;
        let mut traded_qty: u64 = 0;

        for order in RandomFeed::new(seed).take(n) {
            let q = order.qty;
            let events = book.submit(order);
            let rejected = events.iter().any(|e| matches!(e, BookEvent::Rejected { .. }));
            if !rejected {
                accepted_qty += q;
            }
            for ev in events {
                if let BookEvent::Trade { qty, .. } = ev {
                    traded_qty += qty;
                }
            }
        }

        let resting: u64 = book.bids.values().map(|l| l.total_qty).sum::<u64>()
            + book.asks.values().map(|l| l.total_qty).sum::<u64>();

        prop_assert_eq!(accepted_qty, resting + 2 * traded_qty);
    }

    #[test]
    fn cancel_removes_order(seed: u64, n in 1usize..200) {
        let mut book = OrderBook::new();

        for order in RandomFeed::new(seed).take(n) {
            book.submit(order);
        }

        let resting_ids: Vec<_> = book.index.keys().copied().collect();
        for id in resting_ids {
            let result = book.cancel(id);
            let canceled = matches!(result, Some(BookEvent::Canceled { .. }));
            prop_assert!(canceled);
            prop_assert!(!book.index.contains_key(&id));
            for level in book.bids.values().chain(book.asks.values()) {
                prop_assert!(level.orders.iter().all(|o| o.id != id));
            }
        }
    }

    #[test]
    fn level_total_qty_matches_sum(seed: u64, n in 1usize..500) {
        let mut book = OrderBook::new();
        for order in RandomFeed::new(seed).take(n) {
            book.submit(order);
        }
        for level in book.bids.values().chain(book.asks.values()) {
            let sum: u64 = level.orders.iter().map(|o| o.qty).sum();
            prop_assert_eq!(level.total_qty, sum);
        }
    }
}
