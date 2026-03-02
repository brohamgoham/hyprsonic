use hypersdk::{Decimal, hypercore::L2Book};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SuiSonic {
    pub bid_depth: Decimal,
    pub ask_depth: Decimal,
    pub imbalance_rt: Decimal,
    pub time: u64,
    pub signal: Signal,
}

#[derive(Debug, Default, Hash, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub enum Signal {
    BidHeavy,
    AskHeavy,
    #[default]
    Balanced,
}
impl SuiSonic {
    pub fn parse_l2book(&mut self, book: L2Book) -> SuiSonic {
        let big_heavy: Decimal = Decimal::new(15, 1);
        let ask_heavy: Decimal = Decimal::new(67, 2);
        let bid_depth = book
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
        let imbalance_rt = bid_depth / ask_depth;

        let signal = if imbalance_rt > big_heavy {
            Signal::BidHeavy
        } else if imbalance_rt < ask_heavy {
            Signal::AskHeavy
        } else {
            Signal::Balanced
        };
        SuiSonic {
            bid_depth,
            ask_depth,
            imbalance_rt,
            time,
            signal,
        }
    }
}
