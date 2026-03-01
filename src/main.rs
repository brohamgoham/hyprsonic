use anyhow::Result;
use futures::StreamExt;
use hyperliquid_rust_sdk::ExchangeClient;
use hypersdk::{Decimal, hypercore::*};
use std::fmt::Debug;
use std::time::Duration;
use std::{collections::HashMap, hash::Hash, process::Termination};

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

    // let mut sonic = HashMap::new();
    let mut stream = client.websocket();

    stream.subscribe(Subscription::Trades {
        coin: "ETH".to_string(),
    });
    stream.subscribe(Subscription::Trades {
        coin: "BTC".to_string(),
    });
    stream.subscribe(Subscription::Trades {
        coin: "SUI".to_string(),
    });
    stream.subscribe(Subscription::Bbo {
        coin: "HYPE".to_string(),
    });
    stream.subscribe(Subscription::L2Book {
        coin: "HYPE".to_string(),
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
            Incoming::Bbo(b) => {
                println!("BBO | Asset {} | BBO {:#?}", b.coin, b.bbo)
            }
            Incoming::L2Book(l) => {
                println!("L2 Book | Asset {} | Levels {:#?} ", l.coin, l.levels)
            }
            _ => {}
        };
    }

    let emerald = client.websocket();

    Ok(())
}
