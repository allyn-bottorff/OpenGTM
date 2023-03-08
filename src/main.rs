pub mod gtm;

use warp::Filter;

use reqwest;
use tokio;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting health checkers");

    let healthz = warp::path("healthz").map(|| "OK");
    let livez = warp::path("livez").map(|| "OK");

    let routes = warp::get().and(healthz.or(livez));

    tokio::spawn(warp::serve(routes).run(([127, 0, 0, 1], 8080)));

    loop {
        let resp = reqwest::get("http://127.0.0.1:9090/ping").await?.text().await?;
        println!("{}", resp);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_the_tests() {}
}
