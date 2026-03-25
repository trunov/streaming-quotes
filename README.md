# streaming-quotes

Real-time stock quote streaming system over TCP/UDP in Rust. Server generates synthetic market data via random walk and streams filtered quotes to subscribed clients, with ping/pong keep-alive and shared-state concurrency using `Arc<Mutex>`.

## Architecture

```
                    TCP                          UDP
Client ──────────────────→ Server    Server ═══════════→ Client
       "STREAM udp://..."            quotes (filtered)

Client ═══════════════════════════════════════════════→ Server
                          UDP "PING" (keep-alive)
```

- **TCP** — control channel. Client sends a single `STREAM` command to subscribe.
- **UDP** — data channel. Server pushes quotes, client sends periodic PINGs.
- Server removes clients that stop pinging within 5 seconds.

### Threading model

- **Generator thread** — produces quotes for all tickers every 500ms, iterates over subscriptions and sends UDP directly.
- **Ping listener thread** — receives UDP `PING` messages and tracks last ping time per client.
- **Cleanup thread** — periodically removes timed-out clients.
- **TCP listener** (main thread) — accepts new connections and registers subscriptions.

Threads communicate via shared `Arc<Mutex<Vec<ClientSubscription>>>` and `Arc<Mutex<HashMap<SocketAddr, Instant>>>`. Locks are never held during blocking I/O to avoid stalls, and are never nested to prevent deadlocks.

> **Note:** An alternative design using crossbeam `bounded` channels with per-client sender threads would provide better isolation (one slow client can't block others) and more idiomatic Rust concurrency. The current shared-state approach was chosen for simplicity.

## Workspace

| Crate | Description |
|---|---|
| `quote-server` | Generates synthetic stock quotes and streams them to subscribers |
| `quote-client` | Connects to the server, receives and displays quotes |

## Protocol

### STREAM command (TCP)

```
STREAM udp://<host>:<port> TICKER1,TICKER2,...
```

Example: `STREAM udp://127.0.0.1:34254 AAPL,TSLA`

### Quote format (UDP)

```
TICKER|PRICE|VOLUME|TIMESTAMP
```

Example: `AAPL|182.35|3200|1711027200000`

### Keep-alive (UDP)

Client sends `PING` every 2 seconds to the server's UDP address (discovered from the source address of the first received packet). Server drops the stream after 5 seconds of silence.

## Usage

### Server

```bash
cargo run -p quote-server
```

Listens on `127.0.0.1:7878` (TCP). Generates quotes for AAPL, GOOGL, TSLA, MSFT, AMZN with random-walk pricing at 500ms intervals.

### Client

```bash
cargo run -p quote-client -- --server 127.0.0.1:7878 --udp-port 34254 --tickers-file tickers.txt
```

| Flag | Default | Description |
|---|---|---|
| `--server`, `-s` | `127.0.0.1:7878` | TCP server address |
| `--udp-port`, `-u` | `34254` | Local port for receiving quotes |
| `--tickers-file`, `-t` | `tickers.txt` | File with tickers, one per line |

### tickers.txt

```
AAPL
GOOGL
TSLA
```