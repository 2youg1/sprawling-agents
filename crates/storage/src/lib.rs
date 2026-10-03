// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Effect side of persistence: Ledger on disk, CAS, in-memory derived
//! views, git checkpoints, queues. Implements kernel ports; holds no
//! policy.

mod error;

pub use error::StorageError;

mod alias;

pub use alias::{AliasKind, WriteTarget};

mod vfs;

mod real_fs;

mod reserved;

mod jsonl;

pub use jsonl::WriteObserver;
pub use jsonl::{CheckedLine, LineCheck, LineFault, read_line};
pub use jsonl::{JsonlLedger, OpenReport, TailTruncation, ledger_segments_at, read_raw_lines_at};
pub use jsonl::{TailLine, TailLines};

// The projection the Ledger lays down beside each building: one file per
// room, disposable, read by nobody in the product (`crates/storage/spec/Sessions.lean` §8-24).
mod sessions;

pub use sessions::Sessions;

#[cfg(any(test, feature = "fault"))]
mod fault_fs;

#[cfg(any(test, feature = "fault"))]
pub use fault_fs::{FaultFs, FaultPlan, TornTail};

mod bundle;

pub use bundle::{Bundle, MANIFEST, Manifest, open_restored};

mod cas;

pub use cas::{BlockOrigin, Cas};

mod index;

pub use index::LedgerIndex;
pub use index::LineReader;
pub use index::Located;
pub use index::Refreshed;

mod hot;

pub use hot::HotView;
pub use hot::RECENT_FROZEN;
pub use hot::RunHot;
pub use hot::RunPhase;
pub use hot::RunWaiting;

mod attribution;

pub use attribution::Attribution;
pub use attribution::AttributionReport;
pub use attribution::Unpriced;

mod queue;

pub use queue::EventQueue;
pub use queue::QueueItem;
pub use queue::QueueLane;

mod chain_audit;

pub use chain_audit::ChainAudit;
pub use chain_audit::ChainHalt;
pub use chain_audit::ProofCount;
pub use chain_audit::Proven;
pub use chain_audit::audit_chain;
pub use chain_audit::prove_chain;

mod verified_prefix;

pub use verified_prefix::ProofRecords;
pub use verified_prefix::line_check_version;

mod snapshot;

pub use snapshot::ChainSnapshot;
pub use snapshot::SnapshotFit;
pub use snapshot::SnapshotStart;
pub use snapshot::StoredSnapshot;
pub use snapshot::WholeFold;
pub use snapshot::read_snapshot;
pub use snapshot::start_from_snapshot;
pub use snapshot::tail_after;
pub use snapshot::write_snapshot;

mod resident;

pub use resident::RESIDENT_TOTAL_BYTES;
pub use resident::Resident;

mod digest_cache;

pub use digest_cache::DigestCache;

mod worktree;

pub use worktree::FileWork;
pub use worktree::Landing;
pub use worktree::PlannedMerge;
pub use worktree::WorktreeLease;
pub use worktree::WorktreeName;
pub use worktree::Worktrees;

mod checkpoint;

pub use checkpoint::ModelChoice;
pub use checkpoint::Provenance;
pub use checkpoint::recorded_effort;
pub use checkpoint::{BaseProgress, Checkpoint};

mod blob;
mod changes;
mod hunks;
mod status;

pub use blob::blob_at;
pub use changes::{Head, between, parents_of};
pub use hunks::{FilePatch, PatchLine, Withheld, of_file};
pub use status::{Drift, WorkingStatus, working_status};
