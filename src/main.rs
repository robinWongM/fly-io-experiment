mod cronet;

use std::time::Duration;

use axum::{routing::get, Router};
use tokio::net::TcpListener;
use tokio::time::sleep;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::thread;

#[tokio::main]
async fn main() {
    // Enable tracing.
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "example_graceful_shutdown=debug,tower_http=debug,axum=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    // Create a regular axum app.
    let app = Router::new()
        .route("/probe", get(|| async {
            let handle = thread::spawn(|| {
                cronet::send_request();
            });
            handle.join().unwrap();
        }))
        .route("/slow", get(|| sleep(Duration::from_secs(5))))
        .route("/forever", get(std::future::pending::<()>))
        .layer((
            TraceLayer::new_for_http(),
            // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
            // requests don't hang forever.
            TimeoutLayer::new(Duration::from_secs(10)),
        ));

    // Create a `TcpListener` using tokio.
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    // Run the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    // return a signal that emits 2 seconds after the program starts.
    tokio::time::sleep(std::time::Duration::from_secs(2000000000000000)).await
}