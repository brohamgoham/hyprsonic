# HYPERSONIC - Personal SUI perp tool

A real time SUI perp market analyzer, you can use any asset for feeds if you would like, just edit the Subscriber in (main.rs)[./src/main.rs]

Subscribes to SUI L2 order book, trades, and BBO streams on Hyperliquid.
On every book update, calculates total bid depth vs ask depth and derives
an imbalance ratio.

## Ingest stream of SUI trades on hype
```
Trades: | Asset SUI | Side Ask | ExecPrice0.90755 | Size 55.1 | TxHash 0x44287a04b37b768145a2043645c9ec02088b00ea4e7e9553e7f12557727f506b
Trades: | Asset SUI | Side Ask | ExecPrice0.90742 | Size 55.1 | TxHash 0x0f5efc141df0983210d8043645c9ed0203ae00f9b8f3b704b327a766dcf4721c
Trades: | Asset SUI | Side Ask | ExecPrice0.90735 | Size 20.7 | TxHash 0x6196fd73da5450576310043645ca2502024c005975576f29055fa8c699582a42
Trades: | Asset SUI | Side Bid | ExecPrice0.90717 | Size 12.8 | TxHash 0x0000000000000000000000000000000000000000000000000000000000000000
Trades: | Asset SUI | Side Ask | ExecPrice0.9072 | Size 165.3 | TxHash 0x2ba922c40b18de5c2d22043645cb1002056100a9a61bfd2ecf71ce16ca1cb846
Trades: | Asset SUI | Side Bid | ExecPrice0.90725 | Size 20.1 | TxHash 0xbe1e21701c106805bf97043645cbe702047e0055b71386d761e6ccc2db1441f0
Trades: | Asset SUI | Side Bid | ExecPrice0.90728 | Size 12.8 | TxHash 0x0000000000000000000000000000000000000000000000000000000000000000
Trades: | Asset SUI | Side Bid | ExecPrice0.90767 | Size 12.8 | TxHash 0x0000000000000000000000000000000000000000000000000000000000000000
Trades: | Asset SUI | Side Bid | ExecPrice0.90758 | Size 23.3 | TxHash 0x2e7b37c4f1810af52ff4043645ce59020d3300aa8c8429c7d243e317b084e4df
Trades: | Asset SUI | Side Bid | ExecPrice0.90735 | Size 12.8 | TxHash 0x0000000000000000000000000000000000000000000000000000000000000000
Trades: | Asset SUI | Side Bid | ExecPrice0.90749 | Size 11.6 | TxHash 0x9f7f76b440f3582ea0f9043645cfba02099d0099dbf6770043482206fff73219
Trades: | Asset SUI | Side Bid | ExecPrice0.90806 | Size 11.5 | TxHash 0x72e526dd3ac797b9745e043645cfcc02055b0101d5cab68b16add22ff9cb71a4
Trades: | Asset SUI | Side Bid | ExecPrice0.90798 | Size 12.8 | TxHash 0x0000000000000000000000000000000000000000000000000000000000000000
Trades: | Asset SUI | Side Ask | ExecPrice0.90791 | Size 40.5 | TxHash 0xd24bfdc097436cd6d3c5043645d0bb02048500a632468ba87614a913564746c1
Trades: | Asset SUI | Side Ask | ExecPrice0.9084 | Size 918.5 | TxHash 0x8e4f945d42f9b3db8fc9043645d1db0202580042ddfcd2ad32183fb001fd8dc6
Trades: | Asset SUI | Side Ask | ExecPrice0.9084 | Size 918.1 | TxHash 0x634a4725231e135764c4043645d1dd020220000abe1132290712f277e211ed42
Trades: | Asset SUI | Side Ask | ExecPrice0.9084 | Size 918.1 | TxHash 0x2e98219189531d463011043645d1dd020224007724563c18d260cce44856f730
```

### Parse L2 book and calculate imbalance ratio of bids/asks from incoming WS stream
```
fn parse_l2book(book: L2Book) -> SuiSonic {}
---
L1: Ask Depth 125132.2 | Signal: Balanced | Total Bid ask 90723.7 | Imbalance RT 0.7250228158699359557332165502 |
L1: Ask Depth 136718.7 | Signal: AskHeavy | Total Bid ask 55227.4 | Imbalance RT 0.4039491305871106147147390957 |
L1: Ask Depth 136070.9 | Signal: AskHeavy | Total Bid ask 55888.5 | Imbalance RT 0.4107307293477150514915386023 |
Trades: | Asset SUI | Side Bid | ExecPrice0.90982 | Size 12.8 | TxHash 0x0000000000000000000000000000000000000000000000000000000000000000
L1: Ask Depth 118645.5 | Signal: AskHeavy | Total Bid ask 54215.1 | Imbalance RT 0.4569503268139120320618986814 |
L1: Ask Depth 117974.1 | Signal: AskHeavy | Total Bid ask 54215.1 | Imbalance RT 0.4595508675209219650753851905 |
L1: Ask Depth 119852.2 | Signal: AskHeavy | Total Bid ask 59163.1 | Imbalance RT 0.4936338256619402897902583348 |
L1: Ask Depth 120401.4 | Signal: AskHeavy | Total Bid ask 59163.1 | Imbalance RT 0.4913821600081062180340095713 |
L1: Ask Depth 111095.8 | Signal: AskHeavy | Total Bid ask 56964.9 | Imbalance RT 0.5127547576055980514114845026 |
L1: Ask Depth 78145.0 | Signal: Balanced | Total Bid ask 59854.9 | Imbalance RT 0.7659466376607588457354917141 |
L1: Ask Depth 81674.1 | Signal: Balanced | Total Bid ask 59663.3 | Imbalance RT 0.7305045295877150773623462028 |
L1: Ask Depth 84020.6 | Signal: Balanced | Total Bid ask 59663.5 | Imbalance RT 0.7101056169558417816583076055 |
L1: Ask Depth 83971.6 | Signal: Balanced | Total Bid ask 59114.0 | Imbalance RT 0.7039761062073367662400144811 |
L1: Ask Depth 73381.0 | Signal: Balanced | Total Bid ask 67801.7 | Imbalance RT 0.923968057126504135947997438 |
```


### More features on the way
- [ ] Spread tracking from BBO stream
- [ ] Volume spike detection from trades stream  
- [ ] Rolling imbalance window (change over time, not just current)
- [ ] ratatui TUI dashboard
- [ ] Crossbeam channels for multi-threaded pipeline
- [ ] Latency measurement per layer
