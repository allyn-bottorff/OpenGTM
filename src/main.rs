pub mod gtm;

use warp::Filter;

use reqwest;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache: Arc<Mutex<HashMap<String, Ipv4Addr>>> = Arc::new(Mutex::new(HashMap::new()));
    println!("Starting health checkers");

    let healthz = warp::path("healthz").map(|| "OK");
    let livez = warp::path("livez").map(|| "OK");

    // Route definition for retriving data out of the cache.
    let t = Arc::clone(&cache);
    let info = warp::path("info").map(move || {
        let ip = t.lock().unwrap()["localhost"].to_string();
        ip
    });

    // Reset the data in the cache to something arbitrary
    let t = Arc::clone(&cache);
    let reset = warp::path("reset").map(move || {
        t.lock()
            .unwrap()
            .insert(String::from("localhost"), Into::into([1, 2, 3, 4]));
        "Reset OK"
    });

    // Create routes object for warp
    let routes = warp::get().and(healthz.or(livez).or(info).or(reset));

    // Run Warp http server in an async process
    tokio::spawn(warp::serve(routes).run(([127, 0, 0, 1], 8080)));

    // Run the "main" loop which calls other apis and updates the cache
    loop {
        // Make an HTTP call to the local pingpong server. Return an error up to the tokio runtime
        // if something goes wrong.
        let resp = reqwest::get("http://127.0.0.1:9090/ping")
            .await?
            .text()
            .await?;
        println!("{}", resp);

        // Grab a lock on the shared data and update the key with the address.
        let t = Arc::clone(&cache);
        {
            let mut ips = t.lock().unwrap();
            ips.insert(String::from("localhost"), Into::into([127, 0, 0, 1]));
        }

        // Sleep to simulate long-ish running operations.
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_the_tests() {}
}
