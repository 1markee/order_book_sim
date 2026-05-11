use std::env;
use std::process;

fn usage() -> ! {
    eprintln!("usage: cc_order_book_sim replay <messages.csv> <orderbook.csv> <depth>");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
    }
    match args[1].as_str() {
        "replay" => {
            if args.len() != 5 {
                usage();
            }
            let msg = &args[2];
            let ob = &args[3];
            let depth: usize = args[4].parse().unwrap_or_else(|_| usage());
            match cc_order_book_sim::replay::replay(msg, ob, depth) {
                Ok(stats) => {
                    println!("processed:  {}", stats.processed);
                    println!("mismatches: {} ({:.4}%)",
                        stats.mismatches,
                        100.0 * stats.mismatches as f64 / stats.processed as f64);
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
        _ => usage(),
    }
}
