mod quotes;
mod server;

use std::net::{TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, Sender};

use quotes::{QuoteGenerator, StockQuote};
use server::ClientSubscription;

use anyhow::{Context, Result};

const PING_TIMEOUT: Duration = Duration::from_secs(5);
const CHANNEL_CAPACITY: usize = 100;

struct ClientHandle {
    tx: Sender<StockQuote>,
    subscription: ClientSubscription,
}

fn main() -> Result<()> {
    let clients: Arc<Mutex<Vec<ClientHandle>>> = Arc::new(Mutex::new(Vec::new()));

    // Generator thread — one for the whole app
    let clients_gen = Arc::clone(&clients);
    thread::spawn(move || {
        let mut quote_gen = QuoteGenerator::new();
        loop {
            let tickers = quote_gen.tickers().to_vec();
            for ticker in &tickers {
                if let Some(quote) = quote_gen.generate_quote(ticker) {
                    let mut handles = clients_gen.lock().unwrap();
                    // Send to all, remove clients whose channel is disconnected
                    handles.retain(|handle| {
                        if handle.subscription.tickers.contains(&quote.ticker) {
                            handle.tx.try_send(quote.clone()).is_ok()
                        } else {
                            true // not interested in this ticker, keep alive
                        }
                    });
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });

    // TCP listener
    let listener = TcpListener::bind("127.0.0.1:7878").context("failed to bind TCP listener")?;
    println!("Server listening on port 7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clients = Arc::clone(&clients);
                thread::spawn(move || {
                    // Parse STREAM command
                    let subscription = match server::handle_client(stream) {
                        Some(sub) => sub,
                        None => return,
                    };

                    let udp_addr = subscription.udp_addr;
                    let tickers = subscription.tickers.clone();
                    println!("Client subscribed: {} -> {:?}", udp_addr, tickers);

                    // Create bounded channel for this client
                    let (tx, rx) = bounded::<StockQuote>(CHANNEL_CAPACITY);

                    // Register with generator
                    clients.lock().unwrap().push(ClientHandle {
                        tx,
                        subscription,
                    });

                    // Per-client sender thread — sends quotes and listens for pings
                    let socket = UdpSocket::bind("0.0.0.0:0")
                        .expect("failed to bind UDP socket");
                    socket
                        .set_read_timeout(Some(Duration::from_millis(50)))
                        .expect("failed to set read timeout");

                    let mut last_ping = Instant::now();
                    let mut buf = [0u8; 64];

                    loop {
                        // Check for PING
                        if let Ok((size, _)) = socket.recv_from(&mut buf) {
                            let msg = String::from_utf8_lossy(&buf[..size]);
                            if msg.trim() == "PING" {
                                last_ping = Instant::now();
                                println!("PING from {}", udp_addr);
                            }
                        }

                        // Ping timeout — client is dead
                        if last_ping.elapsed() > PING_TIMEOUT {
                            println!("Client {} timed out, stopping", udp_addr);
                            return; // rx drops, generator removes this client
                        }

                        // Send queued quotes
                        while let Ok(quote) = rx.try_recv() {
                            let data = quote.serialize();
                            let _ = socket.send_to(data.as_bytes(), udp_addr);
                        }
                    }
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    Ok(())
}