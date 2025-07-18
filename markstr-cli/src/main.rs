//! # Markstr CLI
//!
//! Command-line interface for creating and managing Nostr-based Bitcoin prediction markets.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use markstr_core::{PredictionMarket, utils::*};

#[derive(Parser)]
#[command(name = "markstr")]
#[command(about = "Nostr-based Bitcoin prediction markets using CSFS and Taproot")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new prediction market
    Create {
        /// Market question
        #[arg(short, long)]
        question: String,
        /// Outcome A description
        #[arg(short = 'a', long)]
        outcome_a: String,
        /// Outcome B description
        #[arg(short = 'b', long)]
        outcome_b: String,
        /// Oracle's Nostr public key (hex)
        #[arg(short, long)]
        oracle: String,
        /// Settlement timestamp (Unix timestamp)
        #[arg(short, long)]
        settlement: u64,
    },
    /// Show market information
    Info {
        /// Market ID
        market_id: String,
    },
    /// Generate a new market ID
    GenerateId,
    /// Validate a Bitcoin address
    ValidateAddress {
        /// Bitcoin address to validate
        address: String,
        /// Network (0=Bitcoin, 1=Testnet, 2=Signet, 3=Regtest)
        #[arg(short, long, default_value = "2")]
        network: u8,
    },
    /// Convert between Bitcoin and satoshis
    Convert {
        /// Amount to convert
        amount: f64,
        /// Unit (btc or sat)
        unit: String,
    },
    /// Hash a message with SHA256
    Hash {
        /// Message to hash
        message: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create {
            question,
            outcome_a,
            outcome_b,
            oracle,
            settlement,
        } => {
            println!("{}", "Creating new prediction market...".green().bold());
            
            let market = PredictionMarket::new(
                question.clone(),
                outcome_a.clone(),
                outcome_b.clone(),
                oracle.clone(),
                settlement,
            )?;

            let market_address = market.get_market_address()?;
            
            println!();
            println!("{}", "Market Created Successfully!".green().bold());
            println!("{}", "═".repeat(50).bright_black());
            println!("{}: {}", "Market ID".yellow().bold(), market.market_id);
            println!("{}: {}", "Question".yellow().bold(), question);
            println!("{}: {}", "Outcome A".yellow().bold(), outcome_a);
            println!("{}: {}", "Outcome B".yellow().bold(), outcome_b);
            println!("{}: {}", "Oracle PubKey".yellow().bold(), oracle);
            println!("{}: {}", "Settlement Time".yellow().bold(), format_timestamp(settlement));
            println!("{}: {}", "Network".yellow().bold(), "Signet");
            println!("{}: {}", "Market Address".cyan().bold(), market_address);
            println!("{}: {}", "Status".yellow().bold(), market.get_status());
            println!("{}", "═".repeat(50).bright_black());
            println!();
            println!("{}", "Send bets to the market address above.".bright_blue());
            println!("{}", "Winners will be paid out proportionally after settlement.".bright_blue());
        }
        
        Commands::Info { market_id } => {
            println!("{}", format!("Market Info: {}", market_id).green().bold());
            println!("{}", "This would show stored market information.".yellow());
            println!("{}", "Note: Full market persistence not implemented in this demo.".bright_black());
        }
        
        Commands::GenerateId => {
            let id = generate_market_id();
            println!("{}: {}", "Generated Market ID".green().bold(), id.cyan());
        }
        
        Commands::ValidateAddress { address, network } => {
            let network = u8_to_network(network)?;
            let is_valid = validate_address(&address, network);
            
            if is_valid {
                println!("{}: {} is {} for {}", 
                    "Address Validation".green().bold(),
                    address.cyan(),
                    "valid".green(),
                    format!("{:?}", network).yellow()
                );
            } else {
                println!("{}: {} is {} for {}", 
                    "Address Validation".red().bold(),
                    address.cyan(),
                    "invalid".red(),
                    format!("{:?}", network).yellow()
                );
            }
        }
        
        Commands::Convert { amount, unit } => {
            match unit.to_lowercase().as_str() {
                "btc" => {
                    let satoshis = btc_to_satoshi(amount);
                    println!("{}: {} BTC = {} satoshis", 
                        "Conversion".green().bold(),
                        amount.to_string().cyan(),
                        satoshis.to_string().yellow()
                    );
                }
                "sat" | "sats" => {
                    let btc = satoshi_to_btc(amount as u64);
                    println!("{}: {} satoshis = {} BTC", 
                        "Conversion".green().bold(),
                        (amount as u64).to_string().cyan(),
                        btc.to_string().yellow()
                    );
                }
                _ => {
                    println!("{}: Unit must be 'btc' or 'sat'", "Error".red().bold());
                }
            }
        }
        
        Commands::Hash { message } => {
            let hash = sha256_hash(&message);
            println!("{}: {}", "SHA256 Hash".green().bold(), hash.cyan());
        }
    }

    Ok(())
}

/// Print the markstr banner
fn _print_banner() {
    println!("{}", r#"
    ┌─────────────────────────────────────────────────────┐
    │                                                     │
    │  ███╗   ███╗ █████╗ ██████╗ ██╗  ██╗███████╗████████╗██████╗  │
    │  ████╗ ████║██╔══██╗██╔══██╗██║ ██╔╝██╔════╝╚══██╔══╝██╔══██╗ │
    │  ██╔████╔██║███████║██████╔╝█████╔╝ ███████╗   ██║   ██████╔╝ │
    │  ██║╚██╔╝██║██╔══██║██╔══██╗██╔═██╗ ╚════██║   ██║   ██╔══██╗ │
    │  ██║ ╚═╝ ██║██║  ██║██║  ██║██║  ██╗███████║   ██║   ██║  ██║ │
    │  ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝   ╚═╝   ╚═╝  ╚═╝ │
    │                                                     │
    │           Nostr-based Bitcoin Prediction Markets           │
    │                                                     │
    └─────────────────────────────────────────────────────┘
    "#.bright_magenta());
}