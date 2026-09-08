// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where one run stands, settled once the city has agreed to take the
//! work: its rules, its model, who it runs as, and the tree it writes in.

use kernel::{AxError, EventKind};

use crate::effect;

use super::super::{Agreed, Assignment, Given, RunWorker, now_ms, run_id_for};
use super::Site;

impl RunWorker {
    /// Settles where this run stands, once the city has agreed to take
    /// the work.
    ///
    /// What the agreement answered arrives as `agreed` rather than being
    /// asked again: asking twice would put a second authority behind a
    /// credential renewal that may reach the network. What is left is
    /// one phase because it interlocks - a lease is named after the run,
    /// and the run's id is minted after that renewal - so cutting it
    /// apart would move a clock sample, which a structural change may
    /// not relocate.
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
    pub(in crate::assembly) fn stand_up(
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
            adapter,
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
        let run_id = run_id_for(&given.job, addr, now_ms()?);
        // What this run was sent to do, held for as long as anything it
        // raises is still waiting. `run_started` carries the same three
        // facts and a restarted worker folds them from there; this is the
        // live half, registered out of the values the plan below is built
        // from so the two cannot say different things.
        self.governance
            .sent(run_id, &given.task, &given.goal, at.budget);

        // A building under review gives every run its own tree, and the
        // run writes there instead of in the city. Nothing it writes is
        // visible until somebody else checks it — the losing line of the
        // design made physical rather than promised.
        //
        // The fence goes up first: a worktree branches from a commit, so
        // the city needs one before it can lend anything out.
        let mut lease = None;
        if rules.review() {
            memory::Checkpoint::open(&self.city_root)
                .map_err(memory::MemoryError::into_ax)?
                .ensure_base(addr.as_str(), now_ms()?, &who)
                .map_err(memory::MemoryError::into_ax)?;
            let trees =
                memory::Worktrees::open(&self.city_root).map_err(memory::MemoryError::into_ax)?;
            let name = memory::WorktreeName::parse(&run_id.to_string())
                .map_err(memory::MemoryError::into_ax)?;
            let claimed = trees.claim(&name).map_err(memory::MemoryError::into_ax)?;
            self.record_for(
                run_id,
                effect::Line {
                    who: who.to_owned(),
                    addr: addr.clone(),
                    kind: EventKind::WorktreeOpened,
                    data: claimed
                        .opened_payload()
                        .map_err(memory::MemoryError::into_ax)?,
                },
            )?;
            lease = Some(claimed);
        }
        let write_root = lease
            .as_ref()
            .map_or_else(|| self.city_root.clone(), |held| held.path().to_path_buf());
        let branch = lease.as_ref().map(|held| held.name().as_str().to_owned());
        Ok(Site {
            building,
            rules,
            config,
            model,
            adapter: Some(adapter),
            identity,
            who,
            run_id,
            lease,
            write_root,
            branch,
        })
    }
}
