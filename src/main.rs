mod process_data;
mod route_process;

use crate::process_data::ProcessData;
use crate::route_process::SearchParams;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Mutex;
use warp::Filter;

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

    let app_state = Arc::new(AppState {
        process_list: Default::default(),
        tx_sse: tx,
        rx_sse: rx,
    });

    let acquire_process_list = warp::path("acquire_process_list")
        .and(warp::post())
        .and_then({
            // A bit ugly, maybe there exist a more elegant solution
            let state = app_state.clone();
            move || route_process::acquire_process_list(state.clone())
        });

    let processes = warp::path("processes").and(warp::get()).and_then({
        let state = app_state.clone();
        move || route_process::processes(state.clone())
    });

    let search = warp::path("search")
        .and(warp::get())
        .and(warp::query::<SearchParams>())
        .and_then({
            let state = app_state.clone();
            move |params| route_process::search(state.clone(), params)
        });

    let data = warp::path("data").and(warp::get()).map({
        let state = app_state.clone();
        move || warp::sse::reply(warp::sse::keep_alive().stream(route_process::data(state.clone())))
    });

    warp::serve(acquire_process_list.or(processes).or(search).or(data))
        .run(([127, 0, 0, 1], 8080))
        .await;
}
