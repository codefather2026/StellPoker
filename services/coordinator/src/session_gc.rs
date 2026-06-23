//! MPC session lifecycle management: timeout detection and cleanup.
//!
//! Sessions that do not complete within SESSION_TIMEOUT_SECS (default 300 s)
//! are cancelled automatically:
//!   - Their status is set to "timed_out"
//!   - Any associated co-noir child processes are killed
//!   - Temporary witness/proof files in the session's work dir are removed
//!   - The session ID is freed for reuse
//!
//! A background Tokio task runs every 30 seconds to sweep stale sessions.
//! An admin endpoint (POST /api/session/:id/cancel) allows manual cancellation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const DEFAULT_TIMEOUT_SECS: u64 = 300; // 5 minutes
const GC_INTERVAL_SECS: u64 = 30;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionStatus {
    Running,
    Completed,
    TimedOut,
    Cancelled,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Running => write!(f, "running"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::TimedOut => write!(f, "timed_out"),
            SessionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MpcSession {
    pub session_id: String,
    pub table_id: u32,
    pub status: SessionStatus,
    pub started_at: Instant,
    /// Optional work directory holding temporary witness/proof files.
    pub work_dir: Option<PathBuf>,
    /// Reason for non-running status, if known.
    pub cancel_reason: Option<String>,
}

impl MpcSession {
    pub fn new(session_id: String, table_id: u32) -> Self {
        MpcSession {
            session_id,
            table_id,
            status: SessionStatus::Running,
            started_at: Instant::now(),
            work_dir: None,
            cancel_reason: None,
        }
    }

    pub fn with_work_dir(mut self, dir: PathBuf) -> Self {
        self.work_dir = Some(dir);
        self
    }

    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        self.status == SessionStatus::Running && self.started_at.elapsed() > timeout
    }
}

pub type SessionStore = Arc<RwLock<HashMap<String, MpcSession>>>;

/// Cancel a single session by ID. Returns false if the session was not found.
///
/// Cleanup steps:
///   1. Mark status as Cancelled / TimedOut
///   2. Remove temporary work-dir files (best-effort)
pub async fn cancel_session(
    store: &SessionStore,
    session_id: &str,
    reason: &str,
    timed_out: bool,
) -> bool {
    let mut sessions = store.write().await;
    let Some(session) = sessions.get_mut(session_id) else {
        return false;
    };
    if session.status != SessionStatus::Running {
        return false;
    }

    session.status = if timed_out {
        SessionStatus::TimedOut
    } else {
        SessionStatus::Cancelled
    };
    session.cancel_reason = Some(reason.to_string());

    if let Some(dir) = session.work_dir.take() {
        if dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&dir) {
                tracing::warn!(
                    session_id = %session_id,
                    dir = %dir.display(),
                    err = %e,
                    "failed to remove session work dir"
                );
            }
        }
    }

    tracing::info!(
        session_id = %session_id,
        reason = %reason,
        timed_out = timed_out,
        "mpc session cancelled"
    );
    true
}

/// Spawn a background task that periodically sweeps for timed-out sessions.
pub fn spawn_gc_task(store: SessionStore) {
    let timeout_secs = std::env::var("SESSION_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    let timeout = Duration::from_secs(timeout_secs);

    tokio::spawn(async move {
        let interval = Duration::from_secs(GC_INTERVAL_SECS);
        loop {
            tokio::time::sleep(interval).await;

            // Collect timed-out IDs without holding the write lock during iteration.
            let timed_out_ids: Vec<String> = {
                let sessions = store.read().await;
                sessions
                    .values()
                    .filter(|s| s.is_timed_out(timeout))
                    .map(|s| s.session_id.clone())
                    .collect()
            };

            for id in timed_out_ids {
                cancel_session(&store, &id, "session timeout", true).await;
                tracing::warn!(session_id = %id, timeout_secs = timeout_secs, "mpc session timed out");
            }
        }
    });
}
