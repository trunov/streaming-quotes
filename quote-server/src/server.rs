use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, UdpSocket};
use std::thread;
use std::time::Duration;

use crate::quotes::QuoteGenerator;

/// Parse "STREAM udp://127.0.0.1:34254 AAPL,TSLA"
fn parse_stream_command(input: &str) -> Option<(String, Vec<String>)> {
    let mut parts = input.split_whitespace();

    let cmd = parts.next()?;
    if cmd != "STREAM" {
        return None;
    }

    let addr = parts.next()?;
    let addr = addr.strip_prefix("udp://")?;

    let tickers: Vec<String> = parts
        .next()?
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .collect();

    if tickers.is_empty() {
        return None;
    }

    Some((addr.to_string(), tickers))
}

pub fn handle_client(stream: TcpStream) {
    let mut writer = stream.try_clone().expect("failed to clone stream");
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => return,
        };

        let input = line.trim().to_string();
        if input.is_empty() {
            continue;
        }

        match parse_stream_command(&input) {
            Some((udp_addr, tickers)) => {
                let _ = writer.write_all(b"OK: streaming started\n");
                let _ = writer.flush();
                println!("Client subscribed: {} -> {:?}", udp_addr, tickers);
                start_udp_stream(udp_addr, tickers);
                return; // one command per connection for now
            }
            None => {
                let _ = writer.write_all(b"ERROR: usage STREAM udp://<host>:<port> TICK1,TICK2\n");
                let _ = writer.flush();
            }
        }
    }
}

fn start_udp_stream(addr: String, tickers: Vec<String>) {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("failed to bind UDP socket");
    let mut quote_gen = QuoteGenerator::new();

    loop {
        for ticker in &tickers {
            if let Some(quote) = quote_gen.generate_quote(ticker) {
                let data = quote.serialize();
                let _ = socket.send_to(data.as_bytes(), &addr);
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_command() {
        let (addr, tickers) = parse_stream_command("STREAM udp://127.0.0.1:34254 AAPL,TSLA").unwrap();
        assert_eq!(addr, "127.0.0.1:34254");
        assert_eq!(tickers, vec!["AAPL", "TSLA"]);
    }

    #[test]
    fn test_parse_single_ticker() {
        let (addr, tickers) = parse_stream_command("STREAM udp://127.0.0.1:9000 GOOGL").unwrap();
        assert_eq!(addr, "127.0.0.1:9000");
        assert_eq!(tickers, vec!["GOOGL"]);
    }

    #[test]
    fn test_parse_invalid_command() {
        assert!(parse_stream_command("GET something").is_none());
        assert!(parse_stream_command("STREAM 127.0.0.1:9000 AAPL").is_none()); // no udp://
        assert!(parse_stream_command("STREAM").is_none());
    }
}