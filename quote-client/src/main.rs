mod args;
mod net;
mod tickers;

use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::Parser;

use anyhow::{Ok, Result, bail};
use args::Args;
use net::{receive_loop, send_stream_command, spawn_ping_thread};
use tickers::read_tickers;

fn main() -> Result<()> {
    let args = Args::parse();

    let tickers = read_tickers(&args.tickers_file);
    if tickers.is_empty() {
        bail!("No tickers found in {}", args.tickers_file);
    }
    println!("Subscribing to: {:?}", tickers);

    send_stream_command(&args.server, args.udp_port, &tickers)?;

    let udp_socket =
        UdpSocket::bind(format!("0.0.0.0:{}", args.udp_port)).expect("failed to bind UDP socket");
    udp_socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .expect("failed to set read timeout");
    println!("Listening for quotes on UDP port {}", args.udp_port);

    let running = Arc::new(AtomicBool::new(true));
    let server_addr = Arc::new(Mutex::new(None));

    let running_ctrlc = Arc::clone(&running);
    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        running_ctrlc.store(false, Ordering::SeqCst);
    })
    .expect("failed to set Ctrl+C handler");

    let ping_socket = udp_socket.try_clone().expect("failed to clone socket");
    let ping_thread =
        spawn_ping_thread(ping_socket, Arc::clone(&server_addr), Arc::clone(&running));

    receive_loop(&udp_socket, &server_addr, &running);

    let _ = ping_thread.join();
    println!("Client stopped");
    Ok(())
}
