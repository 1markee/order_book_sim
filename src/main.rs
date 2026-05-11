use std::env;
use std::process;

use cc_order_book_sim::book::OrderBook;
use cc_order_book_sim::events::BookEvent;
use cc_order_book_sim::feed::random::RandomFeed;
use cc_order_book_sim::order::Side;

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  cc_order_book_sim replay <messages.csv> <orderbook.csv> <depth>");
    eprintln!("  cc_order_book_sim demo [n_orders] [seed]");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
    }
    match args[1].as_str() {
        "replay" => run_replay(&args),
        "demo" => run_demo(&args),
        _ => usage(),
    }
}

fn run_replay(args: &[String]) {
    if args.len() != 5 {
        usage();
    }
    let msg = &args[2];
    let ob = &args[3];
    let depth: usize = args[4].parse().unwrap_or_else(|_| usage());
    match cc_order_book_sim::replay::replay(msg, ob, depth) {
        Ok(stats) => {
            println!("processed:  {}", stats.processed);
            println!(
                "mismatches: {} ({:.4}%)",
                stats.mismatches,
                100.0 * stats.mismatches as f64 / stats.processed as f64
            );
            if let Some(at) = stats.first_mismatch_at {
                println!("first mismatch at message #{}", at);
                if let (Some(ours), Some(theirs)) =
                    (stats.first_mismatch_ours, stats.first_mismatch_lobster)
                {
                    if let Some((i, (a, b))) = ours
                        .iter()
                        .zip(theirs.iter())
                        .enumerate()
                        .find(|(_, (a, b))| a != b)
                    {
                        let level = i / 4 + 1;
                        let field = match i % 4 {
                            0 => "ask_price",
                            1 => "ask_size",
                            2 => "bid_price",
                            _ => "bid_size",
                        };
                        println!(
                            "  col {} (level {} {}): ours={} lobster={}",
                            i, level, field, a, b
                        );
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("replay failed: {}", e);
            process::exit(1);
        }
    }
}

fn run_demo(args: &[String]) {
    let n: usize = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let seed: u64 = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);

    let mut book = OrderBook::new();
    let mut trades = 0u64;

    println!("running {} random orders (seed = {})", n, seed);
    println!();

    for order in RandomFeed::new(seed).take(n) {
        let side = match order.side {
            Side::Bid => "BID",
            Side::Ask => "ASK",
        };
        let price = order
            .price
            .map(|p| p.0.to_string())
            .unwrap_or_else(|| "MKT".into());
        println!("order {:>3}: {} {:>2} @ {}", order.id, side, order.qty, price);

        for event in book.submit(order) {
            match event {
                BookEvent::Trade {
                    maker,
                    taker,
                    price,
                    qty,
                } => {
                    trades += 1;
                    println!(
                        "         → trade: maker={} taker={} qty={} @ {}",
                        maker, taker, qty, price.0
                    );
                }
                BookEvent::Rejected { reason, .. } => {
                    println!("         → rejected: {}", reason);
                }
                BookEvent::Accepted { .. } | BookEvent::Canceled { .. } => {}
            }
        }
    }

    println!();
    print_book(&book, 5);
    println!();
    println!(
        "trades: {}  |  resting orders: {}  |  price levels: {}+{}",
        trades,
        book.index.len(),
        book.bids.len(),
        book.asks.len()
    );
}

fn print_book(book: &OrderBook, depth: usize) {
    println!("top {} levels:", depth);
    println!(
        "  {:>7} {:>5}   {:>7} {:>5}",
        "ask_px", "size", "bid_px", "size"
    );
    println!("  ------- -----   ------- -----");
    let asks: Vec<_> = book.asks.iter().take(depth).collect();
    let bids: Vec<_> = book.bids.iter().rev().take(depth).collect();
    for i in 0..depth {
        let (ap, asz) = match asks.get(i) {
            Some((p, lvl)) => (format!("{}", p.0), format!("{}", lvl.total_qty)),
            None => ("--".into(), "--".into()),
        };
        let (bp, bsz) = match bids.get(i) {
            Some((p, lvl)) => (format!("{}", p.0), format!("{}", lvl.total_qty)),
            None => ("--".into(), "--".into()),
        };
        println!("  {:>7} {:>5}   {:>7} {:>5}", ap, asz, bp, bsz);
    }
}
