# streaming-quotes
A Rust workspace implementing a real-time stock quote streaming system over TCP/UDP. Server generates synthetic market data via random walk and streams filtered quotes to clients, with ping/pong keep-alive and multi-threaded fan-out via mpsc channels.
