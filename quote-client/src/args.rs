use clap::Parser;

#[derive(Parser)]
#[command(name = "quote-client")]
#[command(about = "Stock quote streaming client")]
pub struct Args {
    /// TCP server address
    #[arg(short, long, default_value = "127.0.0.1:7878")]
    pub server: String,

    /// Local UDP port to receive quotes
    #[arg(short, long, default_value_t = 34254)]
    pub udp_port: u16,

    /// Path to tickers file
    #[arg(short, long, default_value = "tickers.txt")]
    pub tickers_file: String,
}
