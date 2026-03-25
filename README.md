# streaming-quotes
A Rust workspace implementing a real-time stock quote streaming system over TCP/UDP. Server generates synthetic market data via random walk and streams filtered quotes to clients, with ping/pong keep-alive and multi-threaded fan-out via mpsc channels.

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

Client sends `PING` every 2 seconds to the server's UDP address (discovered from first received packet). Server drops the stream after 5 seconds of silence.

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