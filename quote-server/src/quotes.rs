use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;

#[derive(Debug, Clone)]
pub struct StockQuote {
    pub ticker: String,
    pub price: f64,
    pub volume: u32,
    pub timestamp: u64,
}

impl StockQuote {
    pub fn serialize(&self) -> String {
        format!(
            "{}|{:.2}|{}|{}",
            self.ticker, self.price, self.volume, self.timestamp
        )
    }

    pub fn _deserialize(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('|').collect();
        if parts.len() == 4 {
            Some(StockQuote {
                ticker: parts[0].to_string(),
                price: parts[1].parse().ok()?,
                volume: parts[2].parse().ok()?,
                timestamp: parts[3].parse().ok()?,
            })
        } else {
            None
        }
    }
}

pub struct QuoteGenerator {
    prices: HashMap<String, f64>,
    tickers: Vec<String>,
}

impl QuoteGenerator {
    pub fn new() -> Self {
        let initial_prices = vec![
            ("AAPL", 180.0),
            ("GOOGL", 140.0),
            ("TSLA", 250.0),
            ("MSFT", 420.0),
            ("AMZN", 185.0),
        ];

        let mut prices = HashMap::new();
        let mut tickers = Vec::new();

        for (ticker, price) in initial_prices {
            prices.insert(ticker.to_string(), price);
            tickers.push(ticker.to_string());
        }

        QuoteGenerator { prices, tickers }
    }

    pub fn tickers(&self) -> &[String] {
        &self.tickers
    }

    pub fn generate_quote(&mut self, ticker: &str) -> Option<StockQuote> {
        let price = self.prices.get_mut(ticker)?;

        // random walk: -1.0 to +1.0
        let mut rng = rand::thread_rng();
        let change = rng.gen_range(-1.0..1.0);
        *price += change;
        if *price < 0.1 {
            *price = 0.1;
        }

        let volume = match ticker {
            "AAPL" | "MSFT" | "TSLA" => rng.gen_range(1000..6000),
            _ => rng.gen_range(100..1100),
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Some(StockQuote {
            ticker: ticker.to_string(),
            price: *price,
            volume,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_creates_all_tickers() {
        let quote_gen = QuoteGenerator::new();
        let tickers = quote_gen.tickers();
        assert_eq!(tickers.len(), 5);
        assert!(tickers.contains(&"AAPL".to_string()));
        assert!(tickers.contains(&"GOOGL".to_string()));
        assert!(tickers.contains(&"TSLA".to_string()));
        assert!(tickers.contains(&"MSFT".to_string()));
        assert!(tickers.contains(&"AMZN".to_string()));
    }

    #[test]
    fn test_generate_quote_known_ticker() {
        let mut quote_gen = QuoteGenerator::new();
        let quote = quote_gen.generate_quote("AAPL");
        assert!(quote.is_some());
        let quote = quote.unwrap();
        assert_eq!(quote.ticker, "AAPL");
        assert!(quote.price > 0.0);
        assert!(quote.volume >= 1000);
        assert!(quote.timestamp > 0);
    }

    #[test]
    fn test_generate_quote_unknown_ticker() {
        let mut quote_gen = QuoteGenerator::new();
        assert!(quote_gen.generate_quote("FAKE").is_none());
    }

    #[test]
    fn test_price_floor() {
        let mut quote_gen = QuoteGenerator::new();
        // force price near zero
        *quote_gen.prices.get_mut("AAPL").unwrap() = 0.05;
        let quote = quote_gen.generate_quote("AAPL").unwrap();
        assert!(quote.price >= 0.1);
    }

    #[test]
    fn test_volume_ranges() {
        let mut quote_gen = QuoteGenerator::new();
        for _ in 0..100 {
            let popular = quote_gen.generate_quote("AAPL").unwrap();
            assert!(popular.volume >= 1000 && popular.volume < 6000);

            let regular = quote_gen.generate_quote("GOOGL").unwrap();
            assert!(regular.volume >= 100 && regular.volume < 1100);
        }
    }
}