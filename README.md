# Rust DEX Aggregator

A high-performance, professional-grade DEX aggregator built in Rust for finding optimal swap routes across decentralized exchanges.

## Features

- **Multi-DEX Support**: UniswapV2, SushiSwap, and other V2 fork protocols
- **Advanced Routing**: Multi-hop path finding with BFS algorithm
- **Smart Optimization**: Multi-criteria scoring (price, gas, slippage)
- **Professional CLI**: Clean output with JSON mode for automation
- **Efficient Caching**: DashMap-based in-memory cache with disk persistence
- **Reusable SDK**: Well-documented library for integration into trading bots
- **Type Safety**: Leverages Rust's type system for reliability

## Architecture

```
┌──────────────────────────┐
│      CLI Layer           │   Built with clap
│  (commands & formatting) │
└──────────┬───────────────┘
           │
┌──────────▼───────────────┐
│   Aggregator SDK         │   Public API
│  - Pool Manager          │
│  - Quote Engine          │
│  - Router                │
│  - Utils                 │
└──────────┬───────────────┘
           │
┌──────────▼───────────────┐
│  Ethereum RPC            │   ethers-rs
│  (Provider + Contracts)  │
└──────────────────────────┘
```

## Installation

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- An Ethereum RPC endpoint (Infura, Alchemy, or local node)

### Build from Source

```bash
git clone https://github.com/yourusername/rust-aggregator
cd rust-aggregator
cargo build --release
```

The binary will be available at `target/release/dex` (or `target/release/dex.exe` on Windows).

### Add to PATH (Optional)

```bash
# Linux/Mac
cp target/release/dex /usr/local/bin/

# Windows (PowerShell as Administrator)
Copy-Item target\release\dex.exe C:\Windows\System32\
```

## Configuration

### 1. Create Environment File

```bash
cp .env.example .env
```

### 2. Edit Configuration

Open `.env` and configure:

```bash
# Required: Your Ethereum RPC endpoint
RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY

# Optional: Network settings
CHAIN_ID=1

# Optional: Factory addresses (defaults provided)
UNISWAP_V2_FACTORY=0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f
SUSHISWAP_FACTORY=0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac

# Optional: Cache settings
CACHE_ENABLED=true
CACHE_TTL_SECONDS=300
CACHE_PATH=./cache/pools.json

# Optional: Routing preferences
MAX_HOPS=3
DEFAULT_SLIPPAGE_BPS=50
GAS_PRICE_GWEI=30
```

## Usage

### Fetch Pools

Fetch pool data from a DEX factory:

```bash
# Fetch from Uniswap V2 factory
dex fetch-pools \
  --factory 0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f \
  --name Uniswap \
  --limit 100

# Fetch from SushiSwap factory
dex fetch-pools \
  --factory 0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac \
  --name SushiSwap \
  --limit 50
```

### Get Swap Quotes

Get the best quote for a token swap:

```bash
# Basic quote (using token addresses)
dex quote \
  0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 \
  0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  1.0

# With optimization strategy
dex quote \
  0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 \
  0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  1.0 \
  --optimize price      # Options: price, gas, slippage, balanced

# JSON output for scripting
dex quote \
  0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 \
  0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  1.0 \
  --json
```

### List Cached Pools

```bash
# List all pools
dex list-pools

# Filter by token
dex list-pools --token 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2

# JSON output
dex list-pools --json
```

### Cache Management

```bash
# Export cache to file
dex cache export ./my-pools.json

# Import cache from file
dex cache import ./my-pools.json

# Show statistics
dex cache stats

# Clear cache
dex cache clear
```

## Optimization Strategies

### Price (default)
Maximizes output amount, ignoring gas costs. Best for large trades.

```bash
dex quote [TOKEN_IN] [TOKEN_OUT] [AMOUNT] --optimize price
```

### Gas
Minimizes gas costs, may sacrifice some output. Best for small trades where gas is significant.

```bash
dex quote [TOKEN_IN] [TOKEN_OUT] [AMOUNT] --optimize gas
```

### Slippage
Minimizes price impact, prioritizes low-slippage routes. Best for large trades in shallow markets.

```bash
dex quote [TOKEN_IN] [TOKEN_OUT] [AMOUNT] --optimize slippage
```

### Balanced
Smart balance between price, gas, and slippage. Recommended for most use cases.

```bash
dex quote [TOKEN_IN] [TOKEN_OUT] [AMOUNT] --optimize balanced
```

## SDK Usage

### Basic Example

```rust
use rust_aggregator::{Aggregator, Config, OptimizationStrategy};
use ethers::types::{Address, U256};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::from_env()?;
    
    // Create aggregator
    let aggregator = Aggregator::new(config).await?;
    
    // Fetch pools
    let factory = Address::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f")?;
    aggregator.fetch_pools(factory, "Uniswap".to_string(), Some(100)).await?;
    
    // Get quote
    let token_in = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    let token_out = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    let amount = U256::from_dec_str("1000000000000000000")?; // 1 ETH
    
    let quote = aggregator.get_best_quote(
        token_in,
        token_out,
        amount,
        OptimizationStrategy::Balanced,
    )?;
    
    println!("Best route: {}", quote.description);
    println!("Output: {}", quote.amount_out);
    
    Ok(())
}
```

### Advanced Usage

```rust
use rust_aggregator::{Aggregator, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let aggregator = Aggregator::new(config).await?;
    
    // Import cached pools
    aggregator.import_cache("./cache/pools.json")?;
    
    // Get cache statistics
    let stats = aggregator.get_cache_stats();
    println!("Total pools: {}", stats.total_pools);
    
    // Get pools for specific token
    let token = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    let pools = aggregator.get_pools_with_token(token);
    println!("Found {} pools with WETH", pools.len());
    
    Ok(())
}
```

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_uniswap_v2_formula

# Run with verbose logging
cargo test -- --nocapture --test-threads=1
```

## Performance

- Pool fetching: ~10-50 pools/second (RPC dependent)
- Route calculation: <10ms for 1000+ pools
- Multi-hop routing: Up to 4 hops with BFS
- Memory usage: ~5KB per cached pool

## Roadmap

- [ ] Phase 1: UniswapV2 support (Current)
- [ ] Phase 2: UniswapV3 concentrated liquidity
- [ ] Phase 3: Curve and Balancer support
- [ ] Phase 4: Split routing optimization
- [ ] Phase 5: MEV protection features
- [ ] Phase 6: TUI mode with ratatui
- [ ] Phase 7: Historical analytics
- [ ] Phase 8: REST API server

## Common Token Addresses (Ethereum Mainnet)

```
WETH:  0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2
USDC:  0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
USDT:  0xdAC17F958D2ee523a2206206994597C13D831ec7
DAI:   0x6B175474E89094C44Da98b954EedeAC495271d0F
WBTC:  0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599
```

## Troubleshooting

### "RPC_URL not set" Error

Create a `.env` file with your RPC endpoint:
```bash
cp .env.example .env
# Edit .env and add your RPC URL
```

### "No pools cached" Error

Fetch pools before getting quotes:
```bash
dex fetch-pools --factory 0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f --limit 100
```

### Slow Pool Fetching

- Use `--limit` to fetch fewer pools
- Consider using cache import/export for faster subsequent runs
- Check your RPC rate limits

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

## License

MIT License - see LICENSE file for details

## Disclaimer

This software is for educational purposes. Always verify trades before execution. The authors are not responsible for any financial losses.
