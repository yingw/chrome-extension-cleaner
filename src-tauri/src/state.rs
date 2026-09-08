//! Process-scoped UI state: baselines, scan snapshots, plans, session totals.
//!
//! Snapshots bind candidate ids to full records for exactly one scan.
//! A new scan replaces the snapshot AND drops all plans, so stale
//! candidate ids can never resolve to a path afterwards.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;

/// Server-side record behind one candidate id. The absolute path
/// never leaves this struct.
#[derive(Debug, Clone)]
pub struct CandidateRecord {
    pub path: PathBuf,
    pub size: u64,
    pub profile: String,
    pub ext_id: String,
    pub name: String,
    pub names: HashMap<String, String>,
    pub active_version: Option<String>,
    pub version: String,
    pub dir: String,
    pub reason: String,
}

/// Candidate id -> record, valid for a single scan only.
pub type Snapshot = HashMap<String, CandidateRecord>;

/// One item inside a cleanup plan. `path` is skipped in serialization:
/// the frontend sees everything except where it lives on disk.
#[derive(Debug, Clone, Serialize)]
pub struct PlanItem {
    pub cid: String,
    pub profile: String,
    pub ext_id: String,
    pub name: String,
    pub names: HashMap<String, String>,
    pub active_version: Option<String>,
    pub version: String,
    pub dir: String,
    pub size: u64,
    pub reason: String,
    #[serde(skip)]
    pub path: PathBuf,
}

/// A prepared, not-yet-executed cleanup. Single use per item.
#[derive(Debug)]
pub struct CleanupPlan {
    pub id: String,
    pub scan_id: u64,
    pub items: Vec<PlanItem>,
    pub used: HashSet<String>,
}

struct Inner {
    baseline: Option<(u64, u64)>,
    scan_seq: u64,
    current_scan: Option<u64>,
    snapshots: HashMap<u64, Snapshot>,
    plans: HashMap<String, CleanupPlan>,
    session_freed: u64,
    /// Number of version folders removed this session. Counts folders, not
    /// "Start cleaning" presses: one press can remove many folders, and the
    /// card must match what the review list showed row by row.
    session_cleaned: u64,
}

pub struct AppState {
    inner: Mutex<Inner>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                baseline: None,
                scan_seq: 0,
                current_scan: None,
                snapshots: HashMap::new(),
                plans: HashMap::new(),
                session_freed: 0,
                session_cleaned: 0,
            }),
        }
    }
}

impl AppState {
    /// Allocate a fresh scan id.
    pub fn next_scan_id(&self) -> u64 {
        let mut inner = self.inner.lock().unwrap();
        inner.scan_seq += 1;
        inner.scan_seq
    }

    /// Publish a snapshot. Drops older snapshots AND all plans:
    /// ids from a previous scan stop resolving immediately.
    pub fn store_snapshot(&self, scan_id: u64, snapshot: Snapshot) {
        let mut inner = self.inner.lock().unwrap();
        inner.snapshots.clear();
        inner.plans.clear();
        inner.snapshots.insert(scan_id, snapshot);
        inner.current_scan = Some(scan_id);
    }

    /// Cloned snapshot of the current scan, if any.
    pub fn snapshot(&self) -> Option<(u64, Snapshot)> {
        let inner = self.inner.lock().unwrap();
        let scan_id = inner.current_scan?;
        inner.snapshots.get(&scan_id).map(|s| (scan_id, s.clone()))
    }

    /// First-scan totals, captured once per process. Raw bytes: the
    /// frontend owns all number formatting.
    pub fn baseline_or_set(&self, total_bytes: u64, waste_bytes: u64) -> (u64, u64) {
        let mut inner = self.inner.lock().unwrap();
        *inner.baseline.get_or_insert((total_bytes, waste_bytes))
    }

    pub fn insert_plan(&self, plan: CleanupPlan) {
        self.inner.lock().unwrap().plans.insert(plan.id.clone(), plan);
    }

    /// Take one plan item for execution. Marks it used first, so neither
    /// replays nor concurrent double-takes can delete twice. Returns the
    /// item only if its plan belongs to the current scan.
    pub fn take_plan_item(&self, plan_id: &str, cid: &str) -> Result<PlanItem, String> {
        let mut inner = self.inner.lock().unwrap();
        let current = inner.current_scan;
        let plan = inner
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| "unknown or expired plan; rescan and prepare again".to_string())?;
        if Some(plan.scan_id) != current {
            return Err("plan belongs to an older scan; rescan and prepare again".to_string());
        }
        if !plan.used.insert(cid.to_string()) {
            return Err("candidate already executed; replay refused".to_string());
        }
        plan.items
            .iter()
            .find(|item| item.cid == cid)
            .cloned()
            .ok_or_else(|| "candidate not part of this plan".to_string())
    }

    /// Add to the freed byte total without counting a removal. Used on the
    /// failure path, which still wants to report the running session total.
    pub fn add_session_freed(&self, bytes: u64) -> u64 {
        let mut inner = self.inner.lock().unwrap();
        inner.session_freed += bytes;
        inner.session_freed
    }

    /// Record one successfully removed version folder and return the running
    /// session totals as `(bytes_freed, folders_removed)`.
    pub fn record_deletion(&self, bytes: u64) -> (u64, u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.session_freed += bytes;
        inner.session_cleaned += 1;
        (inner.session_freed, inner.session_cleaned)
    }

    /// Running session totals as `(bytes_freed, folders_removed)`. Lets the
    /// scan payload report them too, so the cards have one source of truth
    /// instead of relying on values left over in the DOM.
    pub fn session_stats(&self) -> (u64, u64) {
        let inner = self.inner.lock().unwrap();
        (inner.session_freed, inner.session_cleaned)
    }
}
