use crate::process_data::ProcessData;
use crate::AppState;
use futures::{Stream, TryStreamExt};
use serde::Deserialize;
use std::convert::Infallible;
use std::ops::Deref;
use std::sync::Arc;
use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, RefreshKind, SystemExt, UserExt};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use warp::reply::json;
use warp::sse::Event;
use warp::Reply;

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

pub async fn acquire_process_list(app_state: Arc<AppState>) -> Result<impl Reply, Infallible> {
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

    Ok(warp::http::StatusCode::OK)
}

pub async fn processes(app_state: Arc<AppState>) -> Result<impl Reply, Infallible> {
    // let process_list = app_state.get_process_list().await;

    Ok(json(app_state.process_list.lock().await.deref()))
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pid: Option<usize>,
    username: Option<String>,
}

pub async fn search(
    app_state: Arc<AppState>,
    params: SearchParams,
) -> Result<impl Reply, Infallible> {
    let mut process_list = app_state.get_process_list().await;

    if let Some(pid) = params.pid {
        process_list.retain(|p| p.pid == pid);
    }

    if let Some(username) = params.username {
        process_list.retain(|p| p.username == username);
    }

    Ok(json(&process_list))
}

pub fn data(app_state: Arc<AppState>) -> impl Stream<Item = Result<Event, Infallible>> {
    BroadcastStream::new(app_state.rx_sse.resubscribe())
        .into_stream()
        .filter_map(|r| r.ok())
        .map(|p| Ok(Event::default().json_data(p).unwrap_or_default())) // Should change this unwrap (maybe by using the anyhow crate)
}
