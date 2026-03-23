use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ClientSubscription {
    pub udp_addr: SocketAddr,
    pub tickers: HashSet<String>,
}

fn parse_stream_command(input: &str) -> Option<(SocketAddr, HashSet<String>)> {
    let mut parts = input.split_whitespace();

    if parts.next()? != "STREAM" {
        return None;
    }

    let addr: SocketAddr = parts.next()?.strip_prefix("udp://")?.parse().ok()?;
    if addr.ip().is_unspecified() {
        return None;
    }

    let tickers: HashSet<String> = parts
        .next()?
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .collect();

    if tickers.is_empty() {
        return None;
    }

    Some((addr, tickers))
}

pub fn handle_client(
    stream: TcpStream,
    clients: Arc<Mutex<Vec<ClientSubscription>>>,
) {
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

                clients.lock().unwrap().push(ClientSubscription {
                    udp_addr,
                    tickers,
                });

                return;
            }
            None => {
                let _ = writer.write_all(
                    b"ERROR: usage STREAM udp://<host>:<port> TICK1,TICK2\n",
                );
                let _ = writer.flush();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_command() {
        let (addr, tickers) =
            parse_stream_command("STREAM udp://127.0.0.1:34254 AAPL,TSLA").unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:34254");
        assert!(tickers.contains("AAPL"));
        assert!(tickers.contains("TSLA"));
    }

    #[test]
    fn test_parse_single_ticker() {
        let (addr, tickers) =
            parse_stream_command("STREAM udp://127.0.0.1:9000 GOOGL").unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:9000");
        assert!(tickers.contains("GOOGL"));
        assert_eq!(tickers.len(), 1);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_stream_command("GET something").is_none());
        assert!(parse_stream_command("STREAM 127.0.0.1:9000 AAPL").is_none());
        assert!(parse_stream_command("STREAM").is_none());
        assert!(parse_stream_command("STREAM udp://0.0.0.0:9000 AAPL").is_none());
    }
}
