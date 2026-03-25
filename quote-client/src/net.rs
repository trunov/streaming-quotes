use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};

pub fn send_stream_command(server: &str, udp_port: u16, tickers: &[String]) -> Result<()> {
    let mut tcp = TcpStream::connect(server).context(format!("failed to connect to {}", server))?;

    let command = format!(
        "STREAM udp://127.0.0.1:{} {}\n",
        udp_port,
        tickers.join(",")
    );
    tcp.write_all(command.as_bytes())
        .context("failed to send command")?;
    tcp.flush()?;

    let mut reader = BufReader::new(&tcp);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .context("failed to read response")?;

    let response = response.trim();
    println!("Server: {}", response);

    if response.starts_with("OK") {
        Ok(())
    } else {
        bail!("server rejected: {}", response)
    }
}

pub fn spawn_ping_thread(
    socket: UdpSocket,
    server_addr: Arc<Mutex<Option<SocketAddr>>>,
    running: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            if let Some(addr) = *server_addr.lock().unwrap() {
                if let Err(e) = socket.send_to(b"PING", addr) {
                    eprintln!("Failed to send PING: {}", e);
                }
            }
            thread::sleep(Duration::from_secs(2));
        }
    })
}

pub fn receive_loop(
    socket: &UdpSocket,
    server_addr: &Arc<Mutex<Option<SocketAddr>>>,
    running: &Arc<AtomicBool>,
) {
    let mut buf = [0u8; 1024];

    while running.load(Ordering::SeqCst) {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                let mut addr = server_addr.lock().unwrap();
                if addr.is_none() {
                    println!("Discovered server UDP address: {}", src);
                    *addr = Some(src);
                }
                drop(addr);

                let data = String::from_utf8_lossy(&buf[..size]);
                println!("{}", data);
            }
            Err(_) => {
                // read timeout, continue
            }
        }
    }
}
