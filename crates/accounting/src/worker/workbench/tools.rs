// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the model may see and what routes what it calls: one
//! registration feeding the catalogue and the bench, the `status` tool
//! that answers what this run is, and the reading room it admits.

use std::sync::{Arc, Mutex};

use kernel::{Address, AxError, Locator};
use runtime::bench::ToolBench;
use runtime::{EditTool, ExecTool, SearchTool, StatusTool};

use super::super::{Assignment, mounts_under};
use super::engine::machine_half;
use super::{BenchDesks, Laying, Reach, Site, Situation, Workbench, held, status_snapshot};

mod kept;
mod ocr;
mod playback;
mod reading_room;
mod transcribe;

impl Laying {
    /// Lays out what the model may see and what routes what it calls.
    ///
    /// It reads the city through [`Laying`] rather than the worker, so it
    /// runs in the lane that drives the run: an MCP server that still
    /// shakes hands holds up that lane and nothing else
    /// (sprawling-SPEC.md 8-113).
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
    pub(in crate::worker) fn lay_out_workbench(
        &self,
        site: &Site,
        desks: &BenchDesks,
        at: &Assignment,
        job_locator: &Locator,
    ) -> Result<Workbench, AxError> {
        let addr = &at.addr;
        // The catalog is the single source of `ChatRequest.tools`: the
        // bench routes a call, the catalog is what the model was told
        // exists, and one registration feeds both.
        //
        // The admitted set is decided here and frozen with the run. It
        // has to be: a provider hashes the tool array ahead of the system
        // prompt, so a tool admitted mid-run would invalidate the whole
        // conversation's cache. Progressive disclosure is about what a
        // line says, not about when a tool appears.
        let catalog = Arc::new(Mutex::new(runtime::Catalog::new()));
        // The mode a run sits in is a capability like any other: until
        // it was set here the mode's own catalog entry reached no model.
        held(&catalog, "lay out the catalog")?.set_mode(at.policy.mode);
        // The building's domain is the ceiling and the dispatch's write
        // limit narrows it; both are asked at every write (city-SPEC 8-32).
        let edit = EditTool::new(
            &site.write_root,
            addr.clone(),
            site.rules.write_domain()?,
            at.policy.write,
        )?;
        // Every tool shares one keeper, so no two of them keep two keys
        // under one name (sprawling-SPEC.md 8-87).
        let keeper = std::sync::Arc::new(kept::Keeper::new(
            Arc::clone(&self.vault),
            self.staged_at.value(),
        ));
        // Who this run can reach, frozen at dispatch: the assembly runs
        // one run at a time and a signal lands after the drive returns,
        // so nothing moves under it. It answers `neighbours` and `status`.
        let seen =
            city::Neighbourhood::scan(&self.city_root, site.building.addr(), addr, &|room| {
                self.waiting.get(room).copied().unwrap_or(0)
            })?;
        // Where this run stands, carried rather than worked out: a run
        // that inferred its own depth would be one wrong answer away
        // from a delegate that delegates.
        let delegates = Arc::new(Mutex::new(collab::DelegateDesk::new(
            at.depth(),
            site.building.addr().clone(),
        )));
        let context = runtime::ContextReading::default();
        let status = self.status_tool(
            site,
            desks,
            at,
            Reach {
                seen: &seen,
                delegates: &delegates,
                context: &context,
            },
        )?;
        let signal_tool = collab::SignalTool::new(Arc::clone(&desks.signals))?;
        let goal_tool = collab::GoalTool::new(addr.clone(), Arc::clone(&desks.goals))?;
        let pr_tool = collab::PrTool::new(addr.clone(), Arc::clone(&desks.pr))?;
        let claim_tool = collab::ClaimTool::new(Arc::clone(&desks.plan))?;
        let archive_tool = collab::ArchiveTool::new(Arc::clone(&desks.shelf))?;
        // The refusal face of the building's own governance. It reaches
        // for the reserved subtree, which no write domain does, and
        // `Effect::Govern` is refused at the effect layer: a run does
        // not change what governs it (city-SPEC section 8-2b).
        let rules_tool = city::RulesTool::new(&self.city_root, site.building.addr().clone())?;
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
        // admitted below this line is still reachable by name. It and
        // `search` ask one read bound, so what one may open the other may
        // find.
        let bound = self.read_bound(site);
        let block_store = kernel::layout::CityLayout::new(&self.city_root).cas();
        let read = runtime::ReadTool::new(
            &site.write_root,
            Arc::clone(&catalog),
            Arc::clone(&bound),
            &block_store,
        )?;
        // The one door the tools that send bytes to an endpoint read
        // through: the same bound, the same tree, the same store as
        // `read`, so what one may open the others may send
        // (runtime-SPEC 8-59).
        let reader = runtime::BoundReader::new(&site.write_root, Arc::clone(&bound), &block_store);
        // Reading needs an address, and until this line there was no way
        // to find one: a symbol had to be hunted through `exec`, which
        // means Python this machine may not have or a shell this
        // building may have switched off. It stands beside `read`
        // because it answers the other half of one question, and it is
        // last in the order for the reason the comment below gives.
        let search = SearchTool::new(&site.write_root, bound)?;
        // The one door out of a filling window. It stands last because
        // it is the newest, and it takes no depth and asks no person:
        // the successor is this same resident, in this same room, with
        // this same table - which is why `delegate` is still on it.
        let succession = Arc::new(Mutex::new(runtime::SuccessionDesk::new()));
        let succeed = runtime::SucceedTool::new(Arc::clone(&succession))?;
        // The net, not the forecast, is the defence (semantic authority
        // 4.4). Two handles on one repository: the bench checkpoints a
        // command its forecast suspects, and the driver checkpoints every
        // wave, so whatever a wave deletes has a commit to come back
        // from. Both stand where the run writes, which is its own tree
        // when the building asks for review.
        // One reading of the write domain feeds both: what the bench
        // admits and what its checkpoint stages are the same set by
        // definition (storage-SPEC section 8-18), and taking them from
        // one call is what keeps them that way.
        let domain = site.rules.write_domain()?;
        let scope: Vec<String> = domain.prefixes().map(|p| p.as_str().to_owned()).collect();
        let mut bench = ToolBench::new(domain)
            .with_checkpoint(runtime::bench::CheckpointNet {
                checkpoint: self.open_checkpoint(&site.write_root)?,
                scope,
                of: site.provenance(self.city, addr),
            })
            .for_job(addr.clone(), job_locator.clone());
        *bench.taint_mut() = at.taint.clone();
        // One registration feeds both. The catalogue is what the model
        // was told exists and the bench is what routes the call it
        // makes, so a name on one list and not the other is either a
        // tool nobody can call or a call nobody was told about; one list
        // leaves nothing to agree by hand.
        //
        // The order is the catalogue's: `render` puts the tools in front
        // of the model in this order and the resident segment is hashed,
        // so this sequence is part of what stays cacheable across a run.
        // A new tool joins at the end for that reason: inserting one in
        // the middle would move every line after it.
        let mut admitted: Vec<Box<dyn kernel::Tool>> = vec![
            Box::new(archive_tool),
            Box::new(claim_tool),
            Box::new(edit),
            Box::new(status),
            Box::new(signal_tool),
            Box::new(goal_tool),
            Box::new(pr_tool),
            Box::new(rules_tool),
            Box::new(neighbours_tool),
            Box::new(read),
            Box::new(search),
            Box::new(succeed),
        ];
        // What this building's residents are for decides the rest of
        // the bench. City Hall writes Markdown and plans: it gets no
        // `exec`, no `delegate` and no `workshop`, and it gets the one
        // door onto the shape of the city instead. Holding that in the
        // tool table rather than in the wording of `MAYOR.md` is the
        // whole point - an invariant a prompt is asked to keep is not
        // an invariant (city-SPEC.md section 8-22).
        //
        // These join at the end of the table for the same reason a new
        // tool does: the order above is hashed with the resident
        // segment, and what keeps its position keeps its cache. Two
        // buildings of different vocations never shared a prefix
        // anyway, because every address in it starts with the building.
        match city::vocation_of(site.building.addr()) {
            city::Vocation::Builds => {
                admitted.push(Box::new(self.exec_tool(site, at)?));
                admitted.push(Box::new(collab::DelegateTool::new(Arc::clone(&delegates))?));
                admitted.push(Box::new(collab::WorkshopTool::new(
                    Arc::clone(&desks.workshop),
                    Arc::clone(&delegates),
                )?));
            }
            city::Vocation::Plans => {
                admitted.push(Box::new(city::CityTool::new(&self.city_root)?));
            }
        }
        // Present only where the book names an endpoint this building
        // may send a recording to (sprawling-SPEC.md 8-131).
        if let Some(transcribe) = self.transcription_tool(site, reader.clone())? {
            admitted.push(Box::new(transcribe));
        }
        // Present only where the book names a model this building may
        // send a picture to (sprawling-SPEC.md 8-142).
        if let Some(ocr) = self.ocr_tool(site, reader)? {
            admitted.push(Box::new(ocr));
        }
        // The last built-in: a look back at the history this building
        // may read, written into the city's playback exports
        // (sprawling-SPEC.md 8-132).
        admitted.push(Box::new(self.playback_tool(site, addr)?));
        admitted.extend(self.outside_tools(site)?);
        for tool in admitted {
            held(&catalog, "lay out the catalog")?.admit_tool(tool.meta())?;
            bench.register(Box::new(kept::Kept::new(
                tool,
                std::sync::Arc::clone(&keeper),
            )))?;
        }
        self.admit_reading_room(&catalog, &site.rules, &site.building, addr)?;
        Ok(Workbench {
            catalog,
            bench: Some(bench),
            delegates,
            succession,
            context,
        })
    }
}

impl Laying {
    /// The tools that reach outside the city, in the order they join the
    /// table after the built-ins: the browsers, then the MCP servers.
    ///
    /// # Errors
    /// Propagates a browser the building's rules ask for and that will
    /// not start.
    fn outside_tools(&self, site: &Site) -> Result<Vec<Box<dyn kernel::Tool>>, AxError> {
        // The browsers this building's rules ask for: its own, then the
        // person's when they declared one. They follow the built-ins,
        // because a provider caches the tool array by position.
        // `city::policy` refuses both settings on a confidential
        // building, so neither is ever reached there.
        let mut outside = (self.browsers)(
            &self.city_root,
            &storage::BlockOrigin {
                run: site.run_id,
                building: site.building.addr().clone(),
            },
            &site.rules,
        )?;
        // External tools, for a building whose configuration names a
        // server or whose rules ask for this machine's desktop. They join
        // the table here, before the catalogue is rendered, because the
        // tool table is frozen with the run: what the model is told
        // exists is decided once.
        for server in self.mcp_tools(
            &self.servers(site),
            &site.write_root,
            site.rules.policy().confidential,
        ) {
            let tool: Box<dyn kernel::Tool> = Box::new(server);
            outside.push(tool);
        }
        Ok(outside)
    }

    /// What this run's building may read by a path its model chose: the read bound, closed over
    /// the building and this city's rules (city-SPEC 8-2).
    ///
    /// Another building's rules are read each time a path lands in it, not here, so a run that
    /// stays in its own building never pays for them, and a building made confidential after this
    /// dispatch is closed from that moment (city-SPEC 12.2). What was read stays for this run
    /// while the file's stamp holds (city-SPEC 12.3).
    fn read_bound(&self, site: &Site) -> runtime::ReadBound {
        let rules = city::RulesCache::new(&self.city_root);
        let home = site.building.addr().clone();
        Arc::new(move |target: &Address| {
            kernel::address::may_read(&home, target, || {
                let holder = city::Building::of(target)?;
                rules.load(holder.addr()).map(|r| r.policy().confidential)
            })
        })
    }

    /// Builds the execution boundary.
    ///
    /// What the run may reach is the frozen configuration's answer;
    /// where the engine and the interpreter live is the machine's, so a
    /// city carried elsewhere does not carry this machine's paths with
    /// it.
    ///
    /// # Errors
    /// Propagates a build with no execution engine and whatever the
    /// tool says about its own construction.
    fn exec_tool(&self, site: &Site, at: &Assignment) -> Result<ExecTool, AxError> {
        let machine = machine_half(&site.config.sandbox, &self.exec_host)?;
        let addr = &at.addr;
        ExecTool::new(
            runtime::ExecSetup {
                workdir: site.write_root.join(addr.as_str()),
                mounts: mounts_under(&site.write_root, &site.config.sandbox.mounts),
                python_wasm: machine.python_wasm,
                shell: machine.shell,
                fuel: runtime::Fuel(site.config.sandbox.fuel),
                // What a child may inherit is the building's own
                // declaration: the four-name floor is enough to run a
                // program and not enough to link one.
                env_passthrough: site.config.sandbox.env_passthrough.clone(),
                domain: addr.clone(),
                run: site.run_id,
                // The dispatch's write limit: under `Create` a command
                // runs only in the copy (runtime-SPEC 8-55).
                limit: at.policy.write,
            },
            machine.engine,
            self.backlog.clone(),
        )
    }

    /// Builds the one tool that answers what this run is, to itself.
    ///
    /// Everything a `status` answer holds is read here and frozen, except
    /// what moves while the run goes on: the children from the delegate
    /// desk, the backlog from its table, the context used from the run's
    /// reading. A borrowed desk answers nothing rather than refusing: a
    /// model told about `status`'s own lock could do nothing about it.
    ///
    /// # Errors
    /// Propagates a write domain that will not resolve and whatever the
    /// tool says about its own construction.
    fn status_tool(
        &self,
        site: &Site,
        desks: &BenchDesks,
        at: &Assignment,
        reach: Reach<'_>,
    ) -> Result<StatusTool, AxError> {
        let watched = Arc::clone(reach.delegates);
        let tool = StatusTool::watching(
            status_snapshot(Situation {
                addr: &at.addr,
                who: &site.who,
                signals_pending: desks.waiting,
                policy: at.policy,
                write_domain: &site.rules.write_domain()?,
                worktree: &site.write_root,
                trust: &self.trust,
                context_tokens: site.model.context_tokens,
                neighbours: reach.seen.residents(),
                // What this resident already holds, so a model asking what it may
                // touch is answered from the same list the conflict check reads.
                locks: self.locks.clone(),
            }),
            Box::new(move || {
                watched.lock().map_or_else(
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
        )?;
        // The thirteenth line: what this run started and left running.
        Ok(tool
            .reporting(self.backlog.clone())
            .metering(reach.context.clone())
            .clocked(site.clock.clone()))
    }
}
