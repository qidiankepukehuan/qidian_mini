use crate::config::AppConfig;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod config;
mod handler;
mod middleware;
mod response;
mod routes;
mod utils;

#[tokio::main]
async fn main() {
    let config = AppConfig::global();
    let app = routes::routers();

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Server running at https://{}", addr);

    axum::serve(listener, app).await.unwrap();
}
