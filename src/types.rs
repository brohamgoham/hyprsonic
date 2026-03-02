use std::collections::HashMap;

use hypersdk::{Decimal, hypercore::L2Book};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SuiSonic {
    pub total_bid_ask: Decimal,
    pub ask_depth: Decimal,
    pub imbalance_rt: Decimal,
    pub time: u64,
    pub signal: Signal,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub enum Signal {
    BidHeavy,
    AskHeavy,
    Balanced,
}

impl SuiSonic {
    pub fn parse_l2book(&mut self, book: L2Book) -> SuiSonic {
        let big_heavy: Decimal = Decimal::new(1, 15);
        let ask_heavy: Decimal = Decimal::new(67, 2);
        let total_bid_ask = book
            .bids()
            .iter()
            .map(|d| d.sz)
            .fold(Decimal::ZERO, |acc, p| acc + p);

        let ask_depth: Decimal = book
            .asks()
            .iter()
            .map(|a| a.sz)
            .fold(Decimal::ZERO, |acc, pp| acc + pp);

        let time = std::time::Instant::now().elapsed().as_secs();
        let imbalance_rt = ask_depth / total_bid_ask;

        let signal = if imbalance_rt > big_heavy {
            Signal::BidHeavy
        } else if imbalance_rt < ask_heavy {
            Signal::AskHeavy
        } else {
            Signal::Balanced
        };
        SuiSonic {
            total_bid_ask,
            ask_depth,
            imbalance_rt,
            time,
            signal,
        }
    }
}
