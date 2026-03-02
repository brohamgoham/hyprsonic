mod types;

use anyhow::Result;
use futures::StreamExt;
use hypersdk::hypercore::*;

use crate::types::SuiSonic;

pub struct HyprSonic {
    pub name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = mainnet();
    match client.chain() {
        Chain::Mainnet => println!("Connected to Mainnet!"),
        Chain::Testnet => println!("testnet initiated"),
    };

    let mut stream = client.websocket();

    stream.subscribe(Subscription::Trades {
        coin: "SUI".to_string(),
    });
 //   stream.subscribe(Subscription::Bbo {
 //       coin: "SUI".to_string(),
 //   });
    stream.subscribe(Subscription::L2Book {
        coin: "SUI".to_string(),
    });
    while let Some(flood) = stream.next().await {
        match flood {
            Incoming::Trades(trades) => {
                for t in trades {
                    println!(
                        "Trades: | Asset {} | Side {} | ExecPrice{} | Size {} | TxHash {}",
                        t.coin, t.side, t.px, t.sz, t.hash
                    )
                }
            }
   //         Incoming::Bbo(b) => {
   //             println!("BBO | Asset {} | BBO {:?}", b.coin, b.bbo)
   //         }
            Incoming::L2Book(l) => {
                let mut sonic = SuiSonic::default();
                let l2 = types::SuiSonic::parse_l2book(&mut sonic, l.clone());
                println!("L1: Ask Depth {} | Signal: {:?} | Total Bid ask {} | Imbalance RT {} |", l2.ask_depth, l2.signal, l2.bid_depth, l2.imbalance_rt);
               // println!("L2 Book | Asset {} | Levels {:?} ", l.coin, l.levels)
            }
            _ => {}
        };
    }

    Ok(())
}
