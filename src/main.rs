mod process_data;
mod route_process;

use crate::process_data::ProcessData;
use axum::routing::{get, post};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct AppState {
    process_list: Mutex<Vec<ProcessData>>,
    tx_sse: Sender<ProcessData>,
    rx_sse: Receiver<ProcessData>,
}

impl AppState {
    async fn get_process_list(&self) -> Vec<ProcessData> {
        self.process_list.lock().await.clone()
    }

    async fn set_process_list(&self, process_list: Vec<ProcessData>) {
        *self.process_list.lock().await = process_list
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let (tx, rx) = tokio::sync::broadcast::channel(100);

    let app = Router::new()
        .route(
            "/acquire_process_list",
            post(route_process::acquire_process_list),
        )
        .route("/processes", get(route_process::processes))
        .route("/search", get(route_process::search))
        .route("/data", get(route_process::data))
        .with_state(Arc::new(AppState {
            process_list: Default::default(),
            tx_sse: tx,
            rx_sse: rx,
        }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
