# Health Checker
Health checking service

Health checking service which performs network based health checks and HTTP API
responses. This is intended to be coupled with a CoreDNS plugin which calls the
API to do dynamic lookups.

# Structure

- Axum API framework
- Tokio-based asynchronous health checking tasks

# Configuration

[source, json]
----
[
    {
        "send": "/health",
        "name": "lbtests",
        "port": 8080,
        "members": [
            "host1.example.com",
            "host2.example.com"
        ],
        "fallback_ip": "127.0.0.0",
        "interval": 30,
        "poll_type": "HTTP"
    },
    {
        "send": "/health",
        "name": "lbtest2",
        "port": 8080,
        "members": [
            "host1.example.com",
            "host2.example.com"
        ],
        "fallback_ip": "127.0.0.0",
        "interval": 30,
        "poll_type": "HTTPS"
    }
]
----

# Running

[source, shell]
----
cargo run
----
Health Checker currently uses https://docs.rs/env_logger/latest/env_logger/[env_logger]
for setting the log level. E.g.

[source, shell]
----
RUST_LOG=info cargo run
----
