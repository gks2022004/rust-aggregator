use clap::{Parser, Subcommand};
use colored::*;
use comfy_table::{presets::UTF8_FULL, Table};
use rust_aggregator::{
    utils, Aggregator, Config, OptimizationStrategy, Result,
};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "dex")]
#[command(about = "DEX Aggregator - Find the best swap routes across multiple DEXs", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output as JSON
    #[arg(short, long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch pools from a DEX factory
    FetchPools {
        /// Factory contract address
        #[arg(long)]
        factory: String,

        /// DEX name (e.g., "Uniswap", "SushiSwap")
        #[arg(long, default_value = "Uniswap")]
        name: String,

        /// Maximum number of pools to fetch
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Get best swap quote
    Quote {
        /// Input token address or symbol
        token_in: String,

        /// Output token address or symbol
        token_out: String,

        /// Amount to swap
        amount: String,

        /// Optimization strategy
        #[arg(long, default_value = "balanced")]
        optimize: String,
    },

    /// List cached pools
    ListPools {
        /// Filter by token address
        #[arg(long)]
        token: Option<String>,
    },

    /// Cache management
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Export cache to file
    Export {
        /// Output file path
        #[arg(default_value = "./cache/pools.json")]
        path: String,
    },

    /// Import cache from file
    Import {
        /// Input file path
        #[arg(default_value = "./cache/pools.json")]
        path: String,
    },

    /// Show cache statistics
    Stats,

    /// Clear cache
    Clear,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize logging
    let level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .without_time()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    // Load configuration
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            eprintln!("\nPlease create a .env file with your RPC_URL.");
            eprintln!("See .env.example for reference.");
            std::process::exit(1);
        }
    };

    // Create aggregator
    let aggregator = match Aggregator::new(config).await {
        Ok(agg) => agg,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    // Execute command
    let result = match cli.command {
        Commands::FetchPools { factory, name, limit } => {
            handle_fetch_pools(&aggregator, &factory, &name, limit, cli.json).await
        }
        Commands::Quote {
            token_in,
            token_out,
            amount,
            optimize,
        } => handle_quote(&aggregator, &token_in, &token_out, &amount, &optimize, cli.json).await,
        Commands::ListPools { token } => handle_list_pools(&aggregator, token.as_deref(), cli.json),
        Commands::Cache { action } => handle_cache(&aggregator, action, cli.json),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

async fn handle_fetch_pools(
    aggregator: &Aggregator,
    factory: &str,
    name: &str,
    limit: Option<usize>,
    json_output: bool,
) -> Result<()> {
    let factory_addr = utils::parse_address(factory)?;

    if !json_output {
        println!("\n{}", "━".repeat(60).bright_cyan());
        println!("{}  {}", "".to_string(), "Fetching Pool Data".bright_cyan().bold());
        println!("{}", "━".repeat(60).bright_cyan());
        println!("  DEX:     {}", name.bright_white().bold());
        println!("  Factory: {}", factory.bright_black());
        println!("  Limit:   {}", limit.map(|l| l.to_string()).unwrap_or_else(|| "All".to_string()).bright_black());
        println!();
    }

    let pools = aggregator.fetch_pools(factory_addr, name.to_string(), limit).await?;

    // Export to cache
    aggregator.export_cache("./cache/pools.json")?;

    if json_output {
        let output = serde_json::json!({
            "success": true,
            "pools_fetched": pools.len(),
            "dex": name,
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("{} {}", "".to_string(), "Success!".bright_green().bold());
        println!("  Pools fetched: {}", pools.len().to_string().bright_yellow().bold());
        println!("  Cache saved:   {}", "./cache/pools.json".bright_cyan());
        println!("{}", "━".repeat(60).bright_cyan());
        println!();
    }

    Ok(())
}

async fn handle_quote(
    aggregator: &Aggregator,
    token_in: &str,
    token_out: &str,
    amount_str: &str,
    optimize: &str,
    json_output: bool,
) -> Result<()> {
    // Parse addresses
    let token_in_addr = utils::parse_address(token_in)?;
    let token_out_addr = utils::parse_address(token_out)?;

    // Get token decimals for proper parsing
    let token_in_decimals = utils::get_token_decimals(token_in_addr);
    let amount_in = utils::parse_token_amount(amount_str, token_in_decimals)?;

    // Parse optimization strategy
    let strategy = match optimize.to_lowercase().as_str() {
        "price" => OptimizationStrategy::Price,
        "gas" => OptimizationStrategy::Gas,
        "slippage" => OptimizationStrategy::Slippage,
        "balanced" => OptimizationStrategy::Balanced,
        _ => OptimizationStrategy::Balanced,
    };

    if !json_output {
        println!("\n{}", "━".repeat(60).bright_cyan());
        println!("{}  {}", "".to_string(), "Searching for Best Route".bright_cyan().bold());
        println!("{}", "━".repeat(60).bright_cyan());
        println!("  Strategy: {}", format!("{:?}", strategy).bright_yellow().bold());
        println!();
    }

    let quote = aggregator.get_best_quote(token_in_addr, token_out_addr, amount_in, strategy)?;

    if json_output {
        let output = serde_json::json!({
            "token_in": format!("{:?}", quote.token_in),
            "token_out": format!("{:?}", quote.token_out),
            "amount_in": quote.amount_in.to_string(),
            "amount_out": quote.amount_out.to_string(),
            "rate": quote.exchange_rate(),
            "hops": quote.hop_count(),
            "gas_estimate": quote.gas_estimate.to_string(),
            "price_impact_bps": quote.price_impact_bps,
            "route": quote.description,
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        print_quote(&quote);
    }

    Ok(())
}

fn handle_list_pools(aggregator: &Aggregator, token_filter: Option<&str>, json_output: bool) -> Result<()> {
    let pools = if let Some(token_str) = token_filter {
        let token_addr = utils::parse_address(token_str)?;
        aggregator.get_pools_with_token(token_addr)
    } else {
        aggregator.get_pools()
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&pools).unwrap());
    } else {
        if pools.is_empty() {
            println!("\n{}", "━".repeat(60).bright_yellow());
            println!("{}  {}", "".to_string(), "No Pools Found".bright_yellow().bold());
            println!("{}", "━".repeat(60).bright_yellow());
            println!("\n  {}", "Tip: Fetch pools first with:".bright_black());
            println!("  {}", "dex fetch-pools --factory 0x5C69... --limit 100".bright_cyan());
            println!();
            return Ok(());
        }

        println!("\n{}", "━".repeat(60).bright_cyan());
        println!("{}  {} - {} pools", "".to_string(), "Cached Pools".bright_cyan().bold(), pools.len().to_string().bright_yellow().bold());
        println!("{}", "━".repeat(60).bright_cyan());
        println!();

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec![
            "DEX".bright_white().bold().to_string(),
            "Token0".bright_white().bold().to_string(),
            "Token1".bright_white().bold().to_string(),
            "Reserve0".bright_white().bold().to_string(),
            "Reserve1".bright_white().bold().to_string(),
        ]);

        for pool in pools.iter().take(20) {
            table.add_row(vec![
                pool.dex_name.bright_cyan().to_string(),
                format!("{:?}", pool.token0)[..10].bright_black().to_string(),
                format!("{:?}", pool.token1)[..10].bright_black().to_string(),
                utils::format_token_amount(pool.reserve0, 18).bright_green().to_string(),
                utils::format_token_amount(pool.reserve1, 18).bright_green().to_string(),
            ]);
        }

        println!("{}", table);

        if pools.len() > 20 {
            println!("\n  {} and {} more pools", "...".bright_black(), (pools.len() - 20).to_string().bright_yellow());
        }
        println!();
    }

    Ok(())
}

fn handle_cache(aggregator: &Aggregator, action: CacheAction, json_output: bool) -> Result<()> {
    match action {
        CacheAction::Export { path } => {
            aggregator.export_cache(&path)?;
            if !json_output {
                println!("\n{} {}", "".to_string(), "Cache Exported".bright_green().bold());
                println!("  Location: {}", path.bright_cyan());
                println!();
            }
        }
        CacheAction::Import { path } => {
            let count = aggregator.import_cache(&path)?;
            if json_output {
                println!("{}", serde_json::json!({"pools_imported": count}));
            } else {
                println!("\n{} {}", "".to_string(), "Cache Imported".bright_green().bold());
                println!("  Pools loaded: {}", count.to_string().bright_yellow().bold());
                println!("  From: {}", path.bright_cyan());
                println!();
            }
        }
        CacheAction::Stats => {
            let stats = aggregator.get_cache_stats();
            if json_output {
                let output = serde_json::json!({
                    "total_pools": stats.total_pools,
                    "dex_counts": stats.dex_counts,
                });
                println!("{}", serde_json::to_string_pretty(&output).map_err(|e| {
                    rust_aggregator::AggregatorError::Other(anyhow::anyhow!("JSON error: {}", e))
                })?);
            } else {
                println!("\n{}", "━".repeat(60).bright_cyan());
                println!("{}  {}", "".to_string(), "Cache Statistics".bright_cyan().bold());
                println!("{}", "━".repeat(60).bright_cyan());
                println!("\n  Total Pools: {}\n", stats.total_pools.to_string().bright_yellow().bold());
                
                if !stats.dex_counts.is_empty() {
                    println!("  {} Pools by DEX:", "".to_string());
                    for (dex, count) in stats.dex_counts {
                        println!("    {} {}", "•".bright_cyan(), format!("{}: {}", dex.bright_white().bold(), count.to_string().bright_yellow()));
                    }
                    println!();
                }
            }
        }
        CacheAction::Clear => {
            aggregator.clear_cache();
            if !json_output {
                println!("\n{} {}", "".to_string(), "Cache Cleared".bright_green().bold());
                println!("  All pools removed from memory");
                println!();
            }
        }
    }

    Ok(())
}

fn print_quote(quote: &rust_aggregator::RouteQuote) {
    // Get decimals and symbols for input and output tokens
    let token_in_decimals = utils::get_token_decimals(quote.token_in);
    let token_out_decimals = utils::get_token_decimals(quote.token_out);
    let token_in_symbol = utils::get_token_symbol(quote.token_in);
    let token_out_symbol = utils::get_token_symbol(quote.token_out);
    
    println!();
    println!("{}", "━".repeat(60).bright_green());
    println!("{}  {}", "".to_string(), "Best Route Found".bright_green().bold());
    println!("{}", "━".repeat(60).bright_green());
    println!();

    // Route visualization with colors and token symbols
    let route_addresses: Vec<&str> = quote.description.split(" → ").collect();
    let route_parts: Vec<String> = route_addresses
        .iter()
        .enumerate()
        .map(|(i, addr)| {
            // Parse address and get symbol
            let token_addr = utils::parse_address(addr).unwrap_or_else(|_| quote.token_in);
            let symbol = utils::get_token_symbol(token_addr);
            
            if i == 0 {
                format!("{} ({})", symbol.bright_cyan().bold(), addr.bright_black())
            } else {
                format!("{} {} ({})", "→".bright_yellow(), symbol.bright_magenta().bold(), addr.bright_black())
            }
        })
        .collect();
    
    println!("  {} {}", "".to_string(), "Route Path:".bright_white().bold());
    println!("    {}", route_parts.join(" "));
    println!();
    println!("  {} {} ", 
        format!("Hops: {}", quote.hop_count()).bright_white().bold(), 
        if quote.hop_count() == 1 { "hop" } else { "hops" }.bright_black()
    );
    println!();

    println!("{}", "━".repeat(60).bright_blue());
    println!("{}  {}", "".to_string(), "Quote Details".bright_blue().bold());
    println!("{}", "━".repeat(60).bright_blue());
    println!();
    
    println!("  {} {} {}", 
        "Input:".bright_white().bold(),
        utils::format_token_amount(quote.amount_in, token_in_decimals).bright_cyan().bold(),
        token_in_symbol.bright_cyan()
    );
    println!("  {} {} {}", 
        "Output:".bright_white().bold(),
        utils::format_token_amount(quote.amount_out, token_out_decimals).bright_green().bold(),
        token_out_symbol.bright_green()
    );
    println!();
    
    // Calculate proper exchange rate using decimals
    let amount_in_f64 = quote.amount_in.as_u128() as f64 / 10f64.powi(token_in_decimals as i32);
    let amount_out_f64 = quote.amount_out.as_u128() as f64 / 10f64.powi(token_out_decimals as i32);
    let rate = if amount_in_f64 > 0.0 {
        amount_out_f64 / amount_in_f64
    } else {
        0.0
    };

    println!("  {} {}", 
        "Rate:".bright_white().bold(), 
        format!("{:.6} {} per {}", rate, token_out_symbol, token_in_symbol).bright_yellow().bold()
    );
    println!();

    println!("{}", "━".repeat(60).bright_magenta());
    println!("{}  {}", "".to_string(), "Cost Analysis".bright_magenta().bold());
    println!("{}", "━".repeat(60).bright_magenta());
    println!();
    
    println!("  {} {} {}", 
        "".to_string(),
        "Gas Estimate:".bright_white().bold(), 
        quote.gas_estimate.to_string().bright_yellow()
    );
    
    let impact = quote.price_impact_bps as f64 / 100.0;
    let impact_color = if impact < 0.5 {
        "green"
    } else if impact < 1.0 {
        "yellow"
    } else {
        "red"
    };
    
    let impact_str = format!("{:.2}%", impact);
    let colored_impact = match impact_color {
        "green" => impact_str.bright_green(),
        "yellow" => impact_str.bright_yellow(),
        "red" => impact_str.bright_red(),
        _ => impact_str.normal(),
    };
    
    println!("  {} {} {}", 
        "".to_string(),
        "Price Impact:".bright_white().bold(), 
        colored_impact.bold()
    );
    
    println!();
    println!("{}", "━".repeat(60).bright_green());
    println!();
}
