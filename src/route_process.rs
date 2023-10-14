use crate::process_data::ProcessData;
use crate::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Sse};
use axum::Json;
use futures::{Stream, StreamExt, TryStreamExt};
use serde::Deserialize;
use std::sync::Arc;
use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, RefreshKind, SystemExt, UserExt};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

fn get_process_list() -> Vec<ProcessData> {
    let mut system = sysinfo::System::new_with_specifics(
        RefreshKind::new()
            .with_processes(ProcessRefreshKind::everything()) // everything may be still too much
            .with_users_list(),
    );
    system.refresh_processes();

    system
        .processes()
        .iter()
        // Maybe should print a warning if a process is skipped ?
        .filter_map(|(pid, process)| {
            let uid = process.user_id()?;

            #[cfg(target_os = "windows")]
            let uid_parsed = uid.to_string();
            #[cfg(not(target_os = "windows"))]
            let uid_parsed = uid.to_string().parse().ok()?;

            Some(ProcessData {
                pid: pid.as_u32() as usize,
                name: process.name().to_string(),
                uid: uid_parsed,
                username: system.get_user_by_id(uid)?.name().to_string(),
            })
        })
        .collect()
}

pub async fn acquire_process_list(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let old_process_list = app_state.get_process_list().await;

    let new_process_list = get_process_list();

    for new_process in new_process_list.iter() {
        if !old_process_list.contains(new_process) {
            if let Err(why) = app_state.tx_sse.send(new_process.clone()) {
                tracing::error!("Error broadcasting: {why}");
            }
        }
    }

    tracing::info!("Refreshed list ({} processes)", new_process_list.len());

    app_state.set_process_list(new_process_list).await;

    StatusCode::OK
}

pub async fn processes(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let process_list = app_state.get_process_list().await;
    Json(process_list)
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pid: Option<usize>,
    username: Option<String>,
}

pub async fn search(
    State(app_state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let mut process_list = app_state.get_process_list().await;

    if let Some(pid) = params.pid {
        process_list.retain(|p| p.pid == pid);
    }

    if let Some(username) = params.username {
        process_list.retain(|p| p.username == username);
    }

    Json(process_list)
}

pub async fn data(
    State(app_state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, BroadcastStreamRecvError>>> {
    let stream = BroadcastStream::new(app_state.rx_sse.resubscribe())
        .into_stream()
        .map(|r| r.map(|p| Event::default().json_data(p).unwrap_or_default())); // Should change this unwrap (maybe by using the anyhow crate)

    Sse::new(stream).keep_alive(KeepAlive::default())
}
