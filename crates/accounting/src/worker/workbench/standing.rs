// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where one run stands, settled once the city has agreed to take the
//! work: its rules, its model, who it runs as, and the tree it writes in.

use std::path::{Path, PathBuf};

use kernel::{Address, AxError, EventKind, RunId};

use crate::effect;

use super::super::{Agreed, Assignment, Given, RunWorker, Stamping, run_id_for};
use super::Site;

/// What a commit this run makes is signed with.
///
/// The city hash arrives rather than being read here: one read of the
/// genesis line serves a whole city, and `RunWorker::city_hash` is where
/// that read happens (`crates/sprawling/Spec.lean` §8-51).
pub(in crate::worker) fn provenance(
    city: kernel::B3Hash,
    addr: &Address,
    run_id: RunId,
    chosen: storage::ModelChoice,
) -> storage::Provenance {
    storage::Provenance::new(run_id, addr.clone(), city, chosen)
}

/// The filter table that governs this run: the building's file over
/// the city's over the built-in three, whole-value (`crates/runtime/Spec.lean` §8-27-6).
///
/// A file that is not there is no layer; a file that cannot be read is
/// reported, because a run sieving under the built-in table while the
/// building wrote its own would be a rule silently unapplied
/// (`crates/sprawling/Spec.lean` §8-26).
///
/// # Errors
/// Propagates a file that exists and cannot be read, and a table that
/// does not parse.
fn filter_table(city_root: &Path, building: &Address) -> Result<runtime::FilterTable, AxError> {
    let layer = |file: PathBuf| -> Result<Option<String>, AxError> {
        match std::fs::read_to_string(&file) {
            Ok(text) => Ok(Some(text)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(AxError::failure(
                kernel::AxCode::StorageFatal,
                "read the filter table",
                format!("{}: {err}", file.display()),
            )
            .with_recovery("make the file readable, or remove it to fall back a layer")),
        }
    };
    // The city's layer is the same rule at the city's own scope, which
    // `CityLayout` states through the reserved subtree and the file
    // name it owns; the building's layer is one call because a building
    // has an address.
    let layout = kernel::layout::CityLayout::new(city_root);
    let city = layer(
        city_root
            .join(kernel::RESERVED_PREFIX)
            .join(kernel::layout::FILTERS_FILE),
    )?;
    let building = layer(layout.filters(building))?;
    runtime::FilterTable::resolve(city.as_deref(), building.as_deref())
}

impl Site {
    /// What this run signs its commits with: its id, the room it works
    /// in, the model it was given and the effort it was asked for.
    ///
    pub(in crate::worker) fn provenance(
        &self,
        city: kernel::B3Hash,
        addr: &Address,
    ) -> storage::Provenance {
        let signed = provenance(
            city,
            addr,
            self.run_id,
            storage::ModelChoice {
                id: self.model.id.clone(),
                effort: self.config.effort,
            },
        );
        match self.predecessor {
            Some(predecessor) => signed.succeeding(predecessor),
            None => signed,
        }
    }
}

/// What placing a room's tree reads from the city, as values rather
/// than as the worker that holds them, so the placement runs on
/// whichever thread prepares the run (`crates/sprawling/Spec.lean` §8-113).
pub(in crate::worker) struct Placing<'a> {
    pub(in crate::worker) city_root: &'a Path,
    pub(in crate::worker) city: kernel::B3Hash,
    /// What time it is, for the base commit a first placement makes.
    pub(in crate::worker) clock: &'a (dyn crate::Clock + Send + Sync),
    /// The city's one checkpoint at a time: a first placement commits the
    /// city's index, which every checkpoint also stages and commits
    /// (`crates/sprawling/Spec.lean` §8-46-13).
    pub(in crate::worker) checkpoint_gate: &'a std::sync::Mutex<()>,
}

impl Site {
    /// Whether this run writes in the room's own tree rather than in the
    /// city: always for an experiment, which lands nothing whatever the
    /// building says, and otherwise when the building asks for review
    /// (`crates/sprawling/Spec.lean` §8-133). The one answer both `name_tree` and
    /// `place_tree` read, so the branch the desks are told and the tree
    /// the run writes in cannot disagree.
    fn works_apart(&self, at: &super::super::Assignment) -> bool {
        match at.policy.landing {
            kernel::LandingPolicy::Experiment => true,
            kernel::LandingPolicy::Ordinary => self.rules.review(),
        }
    }

    /// Lends a room that works apart its tree, kept between the room's
    /// runs, and points this run's writes there instead of at the city:
    /// nothing it writes is visible until somebody else checks it, and
    /// an experiment's never is. A run that does not work apart is left
    /// writing in the city.
    ///
    /// The checkpoint goes up first: a worktree branches from a commit, so
    /// the city needs one before it can lend anything out.
    ///
    /// `worktree_opened` carries the command's key: for a review dispatch
    /// whose rules did not change it is the one line that does, and a
    /// restart recognises the command again from it.
    ///
    /// # Errors
    /// Propagates whatever the checkpoint or the worktree says about
    /// lending a tree out, and the ledger's refusal of the line that
    /// records it.
    pub(in crate::worker) fn place_tree<L: kernel::Ledger>(
        &mut self,
        at: &super::super::Assignment,
        placing: &Placing<'_>,
        lines: &mut Stamping<'_, L>,
    ) -> Result<(), AxError> {
        if !self.works_apart(at) {
            return Ok(());
        }
        let addr = &at.addr;
        // The base commit carries the same trailers every other commit
        // the city makes carries, so the first line of an adopted
        // repository's history already says which session put it there.
        let of = provenance(
            placing.city,
            addr,
            self.run_id,
            storage::ModelChoice {
                id: self.model.id.clone(),
                effort: self.config.effort,
            },
        );
        let claimed = lend_tree(
            &Lending {
                addr,
                building: &self.building,
                run_id: self.run_id,
                who: &self.who,
                of: &of,
            },
            placing,
            lines,
        )?;
        self.write_root = claimed.path().to_path_buf();
        self.branch = Some(claimed.name().as_str().to_owned());
        self.lease = Some(claimed);
        Ok(())
    }
}

/// Whom a room's tree is lent to: the room, the building whose subtree
/// it checks out, the run the `worktree_opened` line is written for,
/// and what the base commit is signed with.
pub(in crate::worker) struct Lending<'a> {
    pub(in crate::worker) addr: &'a Address,
    pub(in crate::worker) building: &'a city::Building,
    pub(in crate::worker) run_id: RunId,
    pub(in crate::worker) who: &'a str,
    pub(in crate::worker) of: &'a storage::Provenance,
}

/// Lends a room its tree, kept between the room's runs: the city's
/// base commit first, because a worktree branches from a commit, then
/// the claim and the line that records it. A review building's run and
/// a harness run take their tree here alike (`crates/sprawling/Spec.lean` §8-124).
///
/// # Errors
/// Propagates whatever the checkpoint or the worktree says about lending
/// a tree out, and the ledger's refusal of the line that records it.
pub(in crate::worker) fn lend_tree<L: kernel::Ledger>(
    lending: &Lending<'_>,
    placing: &Placing<'_>,
    lines: &mut Stamping<'_, L>,
) -> Result<storage::WorktreeLease, AxError> {
    let turn = super::held(placing.checkpoint_gate, "take the checkpoint gate")?;
    storage::Checkpoint::open(placing.city_root)
        .map_err(storage::StorageError::into_ax)?
        .ensure_base(
            &[lending.addr.as_str().to_owned()],
            placing.clock.now()?,
            lending.of,
        )
        .map_err(storage::StorageError::into_ax)?;
    drop(turn);
    let claimed = storage::Worktrees::open(placing.city_root)
        .map_err(storage::StorageError::into_ax)?
        .claim(
            &tree_of(lending.addr)?,
            &super::tree_scope(lending.building),
        )
        .map_err(storage::StorageError::into_ax)?;
    lines.record_for(
        lending.run_id,
        effect::Line {
            who: lending.who.to_owned(),
            addr: lending.addr.clone(),
            kind: EventKind::WorktreeOpened,
            data: claimed
                .opened_payload()
                .map_err(storage::StorageError::into_ax)?,
        },
    )?;
    Ok(claimed)
}

impl Site {
    /// Names the branch a room that works apart writes on before its tree is
    /// placed. The branch is the tree's name, a function of the room
    /// alone, so the desks opened on the accounting thread know it while
    /// the lane still places the tree (`crates/sprawling/Spec.lean` §8-113).
    ///
    /// # Errors
    /// Propagates a room whose tree name will not parse.
    pub(in crate::worker) fn name_tree(
        &mut self,
        at: &super::super::Assignment,
    ) -> Result<(), AxError> {
        if self.works_apart(at) {
            self.branch = Some(tree_of(&at.addr)?.as_str().to_owned());
        }
        Ok(())
    }
}

impl RunWorker {
    /// Settles where this run stands, once the city has agreed to take
    /// the work.
    ///
    /// What the agreement answered arrives as `agreed` rather than being
    /// asked again: asking twice would put a second authority behind a
    /// credential renewal that may reach the network. What is left is
    /// one phase because it interlocks - the run's id is minted after
    /// that renewal - so cutting it apart would move a clock sample,
    /// which a structural change may not relocate. The tree a room under
    /// review writes in is placed afterwards by [`Site::place_tree`],
    /// which needs nothing from this worker (`crates/sprawling/Spec.lean` §8-113).
    ///
    /// The frozen configuration is read here rather than in the
    /// agreement, and the reason is the room: a dispatch that names an
    /// effort writes it into the room's own layer as the room is opened,
    /// so a configuration frozen any earlier would leave this run blind
    /// to the setting it just made.
    ///
    /// # Errors
    /// Propagates configuration that will not load, a resident
    /// description that cannot be read, and whatever the checkpoint or
    /// the worktree says about lending a tree out.
    pub(in crate::worker) fn stand_up(
        &mut self,
        agreed: Agreed,
        at: &Assignment,
        given: &Given,
    ) -> Result<Site, AxError> {
        let addr = &at.addr;
        let Agreed {
            building,
            rules,
            model,
            provider,
            adapter,
            retries,
        } = agreed;
        // City, building and resident layers, resolved once and frozen
        // for the whole run: re-reading them mid-run would let the two
        // halves of one session be shaped by two different settings.
        let config = city::load_config(&self.city_root, addr)?;
        // Who runs this: the address's own URBANITE.md when it has one,
        // and an ephemeral worker when it does not. The identity supplies
        // the resident segment, so the same resident reads the same
        // instructions on every run and the prefix stays cacheable across
        // its whole life.
        let identity = city::Identity::load(&self.city_root, addr)?;
        let who = identity.who();

        // The run's identity is fixed before the tools are built: three
        // of them mint ids from it, and an id minted from a run that did
        // not exist yet would not be the same id on a replay.
        let run_id = run_id_for(&given.job, addr, self.clock.now()?);
        // What this run was sent to do, held for as long as anything it
        // raises is still waiting. `run_started` carries the same three
        // facts and a restarted worker folds them from there; this is the
        // live half, registered out of the values the plan below is built
        // from so the two cannot say different things.
        self.governance.sent(run_id, &given.task, &given.goal);

        let filters = filter_table(&self.city_root, building.addr())?;
        Ok(Site {
            building,
            rules,
            config,
            model,
            provider,
            adapter: Some(adapter),
            identity,
            who,
            run_id,
            predecessor: at.predecessor(),
            lease: None,
            write_root: self.city_root.clone(),
            branch: None,
            filters,
            clock: runtime::ClockReading::default(),
            retries,
        })
    }
}

/// The tree a room under review works in, named after the room so that
/// its next run takes the same tree back. An address may hold any
/// character a tree name may not, so the name carries the leading 16
/// hex digits of the address's digest rather than the address itself.
fn tree_of(addr: &Address) -> Result<storage::WorktreeName, AxError> {
    let digest: String = kernel::B3Hash::digest(addr.as_str().as_bytes())
        .to_string()
        .chars()
        .take(16)
        .collect();
    storage::WorktreeName::parse(&format!("room-{digest}")).map_err(storage::StorageError::into_ax)
}
