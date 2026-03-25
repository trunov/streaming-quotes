mod quotes;
mod server;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::{TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use quotes::QuoteGenerator;
use server::ClientSubscription;

use anyhow::{Context, Result};

const PING_TIMEOUT: Duration = Duration::from_secs(5);

fn main() -> Result<()> {
    let clients: Arc<Mutex<Vec<ClientSubscription>>> = Arc::new(Mutex::new(Vec::new()));
    let last_pings: Arc<Mutex<HashMap<SocketAddr, Instant>>> = Arc::new(Mutex::new(HashMap::new()));

    let udp_socket = UdpSocket::bind("0.0.0.0:0").context("failed to bind UDP socket")?;

    let server_udp_port = udp_socket.local_addr()?;
    println!("UDP sending from {}", server_udp_port);

    // Ping listener thread — reads PINGs from clients on the same socket
    let ping_socket = udp_socket.try_clone()?;
    let last_pings_listener = Arc::clone(&last_pings);
    thread::spawn(move || {
        let mut buf = [0u8; 64];
        loop {
            match ping_socket.recv_from(&mut buf) {
                Ok((size, src)) => {
                    let msg = String::from_utf8_lossy(&buf[..size]);
                    if msg.trim() == "PING" {
                        println!("PING from {}", src);
                        last_pings_listener
                            .lock()
                            .unwrap()
                            .insert(src, Instant::now());
                    }
                }
                Err(e) => {
                    eprintln!("Ping listener error: {}", e);
                }
            }
        }
    });

    // Cleanup thread — removes timed-out clients
    let clients_cleanup = Arc::clone(&clients);
    let last_pings_cleanup = Arc::clone(&last_pings);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            let now = Instant::now();

            // Collect timed-out addresses under pings lock only
            let timed_out: Vec<SocketAddr> = {
                let pings = last_pings_cleanup.lock().unwrap();
                pings
                    .iter()
                    .filter(|(_, last)| now.duration_since(**last) >= PING_TIMEOUT)
                    .map(|(addr, _)| *addr)
                    .collect()
            };

            if !timed_out.is_empty() {
                // lock clients separately
                let mut subs = clients_cleanup.lock().unwrap();
                subs.retain(|sub| !timed_out.contains(&sub.udp_addr));
                drop(subs);

                // Clean up pings
                let mut pings = last_pings_cleanup.lock().unwrap();
                for addr in &timed_out {
                    println!("Client {} timed out, removing", addr);
                    pings.remove(addr);
                }
            }
        }
    });

    // Generator thread
    let clients_gen = Arc::clone(&clients);
    thread::spawn(move || {
        let mut quote_gen = QuoteGenerator::new();
        loop {
            let subscriptions = clients_gen.lock().unwrap().clone();

            let tickers = quote_gen.tickers().to_vec();
            for ticker in &tickers {
                if let Some(quote) = quote_gen.generate_quote(ticker) {
                    let data = quote.serialize();
                    for subscription in &subscriptions {
                        if subscription.tickers.contains(&quote.ticker) {
                            let _ = udp_socket.send_to(data.as_bytes(), subscription.udp_addr);
                        }
                    }
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
                let last_pings = Arc::clone(&last_pings);
                thread::spawn(move || {
                    server::handle_client(stream, clients.clone());

                    // Lock clients, grab what we need, release immediately
                    let last_udp = clients.lock().unwrap().last().map(|x| x.udp_addr);

                    // Lock pings separately — no two locks held at once
                    if let Some(addr) = last_udp {
                        last_pings.lock().unwrap().insert(addr, Instant::now());
                    }
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    Ok(())
}
