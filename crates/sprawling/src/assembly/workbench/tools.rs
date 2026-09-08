// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the model may see and what routes what it calls: one
//! registration feeding the catalogue and the bench, the `status` tool
//! that answers what this run is, and the reading room it admits.

use kernel::{Address, AxError, Locator};
use runtime::bench::ToolBench;
use runtime::{EditTool, ExecTool, StatusTool};

use super::super::{Assignment, PYTHON_WASM_ENV, RunWorker, mounts_under};
use super::engine::{execution_engine, host_shell};
use super::{Desks, Reach, Site, Situation, Workbench, status_snapshot};

impl RunWorker {
    /// Lays out what the model may see and what routes what it calls.
    ///
    /// The catalogue and the bench are one phase because they are one
    /// registration: the catalogue is what the model was told exists,
    /// the bench is what routes the call it makes, and a name on one
    /// list and not the other is either a tool nobody can call or a
    /// call nobody was told about. The delegate desk comes back with
    /// them because two tools hold it and the settlement reads it after
    /// the drive.
    ///
    /// # Errors
    /// Propagates a write domain that will not resolve, a neighbourhood
    /// that cannot be scanned, any tool that refuses to be built, a
    /// duplicate registration on either list, and whatever the reading
    /// room reports.
    pub(in crate::assembly) fn lay_out_workbench(
        &mut self,
        site: &Site,
        desks: &Desks,
        at: &Assignment,
        job_locator: &Locator,
    ) -> Result<Workbench, AxError> {
        let (addr, mode) = (&at.addr, at.mode);
        // The catalog is the single source of `ChatRequest.tools`: the
        // bench routes a call, the catalog is what the model was told
        // exists, and one registration feeds both.
        //
        // The admitted set is decided here and frozen with the run. It
        // has to be: a provider hashes the tool array ahead of the system
        // prompt, so a tool admitted mid-run would invalidate the whole
        // conversation's cache. Progressive disclosure is about what a
        // line says, not about when a tool appears.
        let catalog = std::rc::Rc::new(std::cell::RefCell::new(runtime::Catalog::new()));
        // The mode a run sits in is a capability like any other: it says
        // what this run admits, and until it was set here the mode's own
        // catalog entry reached no model.
        catalog.borrow_mut().set_mode(mode);
        let edit = EditTool::new(&site.write_root, addr.clone(), site.rules.write_domain()?)?;
        // Who this run can reach, read once at dispatch and frozen with
        // it. Nothing here can move under the run: the assembly is
        // single-threaded, so no second run executes while this one
        // drives, and a signal this run sends is delivered after the
        // drive returns. The same value answers the `neighbours` tool
        // and the count `status` reports.
        let seen =
            city::Neighbourhood::scan(&self.city_root, site.building.addr(), addr, &|room| {
                self.inboxes.get(room).map_or(0, collab::Inbox::pending)
            })?;
        // Where this run stands, carried rather than worked out: a run
        // that inferred its own depth would be one wrong answer away
        // from a delegate that delegates.
        let delegates = std::rc::Rc::new(std::cell::RefCell::new(collab::DelegateDesk::new(
            at.depth(),
            site.building.addr().clone(),
        )));
        let status = self.status_tool(
            site,
            desks,
            at,
            Reach {
                seen: &seen,
                delegates: &delegates,
            },
        )?;
        let signal_tool = collab::SignalTool::new(std::rc::Rc::clone(&desks.signals))?;
        let goal_tool = collab::GoalTool::new(addr.clone(), std::rc::Rc::clone(&desks.goals))?;
        let pr_tool = collab::PrTool::new(addr.clone(), std::rc::Rc::clone(&desks.pr))?;
        let claim_tool = collab::ClaimTool::new(std::rc::Rc::clone(&desks.plan))?;
        let delegate_tool = collab::DelegateTool::new(std::rc::Rc::clone(&delegates))?;
        // What this room already got back. Copied rather than lent: the
        // authority is `self.joins`, which is folded from the ledger's
        // handback lines, and a desk that took it away would leave the
        // worker unable to answer the same question after the run.
        let mut held = collab::FanIn::new();
        if let Some(existing) = self.joins.get(addr) {
            for artifact in existing.artifacts() {
                held.accept(artifact.clone());
            }
        }
        let workshop = std::rc::Rc::new(std::cell::RefCell::new(collab::WorkshopDesk::new(
            site.who.clone(),
            held,
        )));
        let workshop_tool = collab::WorkshopTool::new(
            std::rc::Rc::clone(&workshop),
            std::rc::Rc::clone(&delegates),
        )?;
        let archive_tool = collab::ArchiveTool::new(std::rc::Rc::clone(&desks.shelf))?;
        // The one door into the building's own governance. It reaches
        // the reserved subtree, which no write domain does, so it goes
        // through the person rather than through the write gate.
        let rules_tool = city::RulesTool::new(&self.city_root, site.building.addr().clone())?;
        // The execution boundary. What the run may reach is the frozen
        // config's answer; where the engine and the interpreter live is
        // the machine's, so a city carried elsewhere does not carry this
        // machine's paths with it.
        let exec = ExecTool::new(
            site.write_root.join(addr.as_str()),
            mounts_under(&site.write_root, &site.config.sandbox.mounts),
            std::env::var(PYTHON_WASM_ENV)
                .ok()
                .map(std::path::PathBuf::from),
            execution_engine()?,
            if site.config.sandbox.shell {
                host_shell()
            } else {
                None
            },
            runtime::Fuel(site.config.sandbox.fuel),
            addr.clone(),
        )?;
        // The one door onto the rest of the city. It is registered
        // beside `signal` rather than behind it because the two answer
        // different questions - who is there, and what to say to them -
        // and until this line a model could only reach an address
        // somebody had already handed it.
        let neighbours_tool = city::NeighboursTool::new(seen)?;
        // The one tool that reads, and the only caller of the catalog's
        // second-level disclosure: without it a building's reading room
        // could name a skill and never hand it over. It holds the
        // catalog rather than a copy of what is in it, so a skill
        // admitted below this line is still reachable by name.
        let read = runtime::ReadTool::new(&site.write_root, std::rc::Rc::clone(&catalog))?;
        // The net, not the forecast, is the defence (semantic authority
        // 4.4). Two handles on one repository: the bench fences a
        // command its forecast suspects, and the driver fences every
        // wave, so whatever a wave deletes has a commit to come back
        // from. Both stand where the run writes, which is its own tree
        // when the building asks for review.
        let mut bench = ToolBench::new(site.rules.write_domain()?)
            .with_checkpoint(
                memory::Checkpoint::open(&site.write_root).map_err(memory::MemoryError::into_ax)?,
                addr.as_str(),
            )
            .for_job(addr.clone(), job_locator.clone());
        for cluster in &self.governance.granted {
            bench.grant(cluster.clone());
        }
        // One registration feeds both. The catalogue is what the model
        // was told exists and the bench is what routes the call it
        // makes, so a name on one list and not the other is either a
        // tool nobody can call or a call nobody was told about. These
        // used to be two lists of thirteen lines, agreeing by hand.
        //
        // The order is the catalogue's: `render` puts the tools in front
        // of the model in this order and the resident segment is hashed,
        // so this sequence is part of what stays cacheable across a run.
        let mut admitted: Vec<Box<dyn kernel::Tool>> = vec![
            Box::new(archive_tool),
            Box::new(exec),
            Box::new(claim_tool),
            Box::new(edit),
            Box::new(status),
            Box::new(signal_tool),
            Box::new(goal_tool),
            Box::new(pr_tool),
            Box::new(delegate_tool),
            Box::new(workshop_tool),
            Box::new(rules_tool),
            Box::new(neighbours_tool),
            Box::new(read),
        ];
        // External tools, for a building whose configuration names a
        // server. They join the table here, before the catalogue is
        // rendered, because the tool table is frozen with the run: what
        // the model is told exists is decided once.
        for server in self.mcp_tools(
            &site.config,
            &site.write_root,
            site.rules.policy().confidential,
        ) {
            let tool: Box<dyn kernel::Tool> = Box::new(server);
            admitted.push(tool);
        }
        for tool in admitted {
            catalog.borrow_mut().admit_tool(tool.meta())?;
            bench.register(tool)?;
        }
        self.admit_reading_room(&catalog, &site.rules, &site.building, addr)?;
        Ok(Workbench {
            catalog,
            bench,
            delegates,
        })
    }

    /// Builds the one tool that answers what this run is, to itself.
    ///
    /// Everything a `status` answer holds is read here, at dispatch, and
    /// frozen with the tool - except the children, which a closure reads
    /// live from the delegate desk because a run hands work down while
    /// it is going. A borrowed desk answers nothing rather than
    /// refusing: `status` reporting its own plumbing to a model would
    /// teach it about a lock it can do nothing about.
    ///
    /// # Errors
    /// Propagates a write domain that will not resolve and whatever the
    /// tool says about its own construction.
    fn status_tool(
        &self,
        site: &Site,
        desks: &Desks,
        at: &Assignment,
        reach: Reach<'_>,
    ) -> Result<StatusTool, AxError> {
        // What `status.children` reads, and the only part of the answer
        // that is not frozen here.
        let watched = std::rc::Rc::clone(reach.delegates);
        StatusTool::watching(
            status_snapshot(Situation {
                addr: &at.addr,
                who: &site.who,
                signals_pending: desks.waiting,
                mode: at.mode,
                write_domain: &site.rules.write_domain()?,
                worktree: &site.write_root,
                trust: &self.governance.autonomy,
                context_tokens: site.model.context_tokens,
                budget: at.budget,
                neighbours: reach.seen.residents(),
                // What this resident already holds, so a model asking
                // what it may touch is answered from the same list the
                // conflict check reads.
                locks: self
                    .goals
                    .iter()
                    .filter(|entry| entry.owner == site.who)
                    .map(|entry| entry.statement.clone())
                    .collect(),
            }),
            Box::new(move || {
                watched.try_borrow().map_or_else(
                    |_| Vec::new(),
                    |desk| {
                        desk.asked()
                            .iter()
                            .map(|work| runtime::ChildStatus {
                                room: work.room.clone(),
                                kind: work.kind,
                            })
                            .collect()
                    },
                )
            }),
        )
    }

    /// Admits the skills this building's own file names, and says which
    /// of them are not on the shelves.
    ///
    /// The city's shelves may hold a thousand; what costs resident bytes
    /// is the list this building admits. A name on that list which is not
    /// on the shelves is left out rather than promised, and noted so the
    /// person who wrote the name can see it went nowhere.
    ///
    /// # Errors
    /// Propagates a shelf that cannot be read and an entry the catalog
    /// refuses.
    fn admit_reading_room(
        &mut self,
        catalog: &std::rc::Rc<std::cell::RefCell<runtime::Catalog>>,
        rules: &city::BuildingRules,
        building: &city::Building,
        addr: &Address,
    ) -> Result<(), AxError> {
        // The reading room, and only it. The city's shelves may hold a
        // thousand skills; what costs resident bytes is the list this
        // building's own file admits, and a name on that list which is
        // not on the shelves is left out rather than promised.
        let shelves = city::Library::scan(&self.city_root, Some(building.addr()))?;
        for holding in shelves.reading_room(rules.reading_room()) {
            catalog.borrow_mut().admit_skill(runtime::CatalogEntry {
                name: holding.name.clone(),
                disclosure: holding.disclosure.clone(),
                expansion: holding.addr.as_str().to_owned(),
                // What the shelf held at this scan. The run records it,
                // so a document that changes content behind the same
                // name is a difference somebody can see later.
                hash: Some(holding.hash),
            })?;
        }
        for absent in shelves.missing(rules.reading_room()) {
            self.note(
                runtime::diagnostics::Level::Effect,
                "city::library",
                &format!(
                    "{} admits `{absent}`, which is not on the shelves",
                    addr.as_str()
                ),
            );
        }
        Ok(())
    }
}
