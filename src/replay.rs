use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::feed::lobster::MessageRow;
use crate::order::Price;

const ASK_EMPTY: i64 = 9_999_999_999;
const BID_EMPTY: i64 = -9_999_999_999;

#[derive(Debug, Default)]
pub struct LobsterBook {
    pub bids: BTreeMap<Price, u64>,
    pub asks: BTreeMap<Price, u64>,
}

impl LobsterBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, row: MessageRow) {
        let levels = match row.direction {
            1 => &mut self.bids,
            -1 => &mut self.asks,
            _ => return,
        };
        let price = Price(row.price);
        match row.event_type {
            1 => {
                *levels.entry(price).or_insert(0) += row.size;
            }
            2 | 3 | 4 => {
                if let Some(s) = levels.get_mut(&price) {
                    if *s > row.size {
                        *s -= row.size;
                    } else {
                        levels.remove(&price);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn seed_from_snapshot(&mut self, snap: &[i64], depth: usize) {
        for i in 0..depth {
            let ap = snap[i * 4];
            let asz = snap[i * 4 + 1];
            let bp = snap[i * 4 + 2];
            let bsz = snap[i * 4 + 3];
            if ap != ASK_EMPTY && asz > 0 {
                self.asks.insert(Price(ap), asz as u64);
            }
            if bp != BID_EMPTY && bsz > 0 {
                self.bids.insert(Price(bp), bsz as u64);
            }
        }
    }

    pub fn top_n(&self, depth: usize) -> Vec<i64> {
        let mut out = Vec::with_capacity(depth * 4);
        let mut asks = self.asks.iter();
        let mut bids = self.bids.iter().rev();
        for _ in 0..depth {
            match asks.next() {
                Some((p, s)) => {
                    out.push(p.0);
                    out.push(*s as i64);
                }
                None => {
                    out.push(ASK_EMPTY);
                    out.push(0);
                }
            }
            match bids.next() {
                Some((p, s)) => {
                    out.push(p.0);
                    out.push(*s as i64);
                }
                None => {
                    out.push(BID_EMPTY);
                    out.push(0);
                }
            }
        }
        out
    }
}

#[derive(Debug, Default, Clone)]
pub struct ReplayStats {
    pub processed: u64,
    pub mismatches: u64,
    pub first_mismatch_at: Option<u64>,
    pub first_mismatch_ours: Option<Vec<i64>>,
    pub first_mismatch_lobster: Option<Vec<i64>>,
}

pub fn replay<P1, P2>(
    message_path: P1,
    orderbook_path: P2,
    depth: usize,
) -> std::io::Result<ReplayStats>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let mut msg_rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(message_path)?;
    let ob_file = File::open(orderbook_path)?;
    let ob_rdr = BufReader::new(ob_file);

    let mut stats = ReplayStats::default();

    let mut messages = msg_rdr.deserialize::<MessageRow>();
    let mut ob_lines = ob_rdr.lines();

    let _first_msg = messages.next().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "empty message file")
    })??;
    let first_ob = ob_lines.next().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "empty orderbook file")
    })??;
    let mut prev_snapshot = parse_row(&first_ob, depth)?;
    stats.processed = 1;

    loop {
        let (Some(msg_result), Some(ob_result)) = (messages.next(), ob_lines.next()) else {
            break;
        };
        let row = msg_result?;
        let line = ob_result?;
        let snapshot = parse_row(&line, depth)?;

        let mut book = LobsterBook::new();
        book.seed_from_snapshot(&prev_snapshot, depth);
        book.apply(row);
        let our_top = book.top_n(depth);

        stats.processed += 1;
        let verify_cols = (depth - 1) * 4;
        if our_top[..verify_cols] != snapshot[..verify_cols] {
            stats.mismatches += 1;
            if stats.first_mismatch_at.is_none() {
                stats.first_mismatch_at = Some(stats.processed);
                stats.first_mismatch_ours = Some(our_top);
                stats.first_mismatch_lobster = Some(snapshot.clone());
            }
        }
        prev_snapshot = snapshot;
    }

    Ok(stats)
}

fn parse_row(line: &str, depth: usize) -> std::io::Result<Vec<i64>> {
    let parts: Vec<&str> = line.split(',').collect();
    let expected = depth * 4;
    if parts.len() != expected {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("expected {} columns, got {}", expected, parts.len()),
        ));
    }
    parts
        .iter()
        .map(|s| {
            s.trim().parse::<i64>().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
            })
        })
        .collect()
}
