// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use kernel::{Address, AxError, Effect, Locator, ReservedNamespace};
use runtime::bench::ToolBench;
use runtime::{EditTool, Namespace, SearchTool, StatusTool};

use super::super::Assignment;
use super::{
    BenchDesks, ConnectorJob, Laying, Reach, Site, Situation, Workbench, held, status_snapshot,
};

mod endpoints;
mod execution;
mod kept;
mod ocr;
mod playback;
pub(super) mod proposal;
mod reading_room;
mod search;
mod transcribe;

fn nested_rule_candidates(
    site: &Site,
    room: &Address,
    bound: runtime::ReadBound,
) -> runtime::RuleCandidates {
    let scope = crate::prompt_chain::RuleScope {
        city_root: site.write_root.clone(),
        building: site.building.addr().clone(),
        room: room.clone(),
        bound,
    };
    Arc::new(move |admitted, landed, skip, halted| {
        crate::prompt_chain::nested_rules(
            scope.clone(),
            &runtime::RuleContext {
                admitted: admitted.clone(),
                landed: landed.clone(),
            },
            skip,
            halted,
        )
    })
}

impl Laying {
    pub(in crate::worker) fn lay_out_workbench(
        &self,
        site: &Site,
        desks: &BenchDesks,
        at: &Assignment,
        job_locator: &Locator,
    ) -> Result<Workbench, AxError> {
        let addr = &at.addr;

        let catalog = Arc::new(Mutex::new(runtime::Catalog::new()));

        held(&catalog, "lay out the catalog")?.set_mode(at.policy.mode);

        let policy = runtime::PolicyCell::new(at.policy);

        let stamps = runtime::ReadStamps::default();
        let shown = runtime::ReadShown::default();
        let edit = self.edit_tool(site, addr, policy.reader(), stamps.clone())?;

        let keeper = std::sync::Arc::new(kept::Keeper::new(
            Arc::clone(&self.vault),
            self.staged_at.value(),
        ));

        let seen =
            city::Neighbourhood::scan(&self.city_root, site.building.addr(), addr, &|room| {
                self.waiting.get(room).copied().unwrap_or(0)
            })?;
        let delegates = Arc::clone(&desks.delegates);
        let context = runtime::ContextReading::default();
        let status = self.status_tool(
            site,
            desks,
            at,
            Reach {
                seen: &seen,
                delegates: &delegates,
                context: &context,
                catalog: &catalog,
            },
        )?;
        let signal_tool = collab::SignalTool::new(Arc::clone(&desks.signals))?;
        let goal_tool = collab::GoalTool::new(addr.clone(), Arc::clone(&desks.goals))?;
        let pr_tool = collab::PrTool::new(addr.clone(), Arc::clone(&desks.pr))?;
        let archive_tool = desks.archive_tool(&site.write_root)?;

        let rules_tool = city::RulesTool::new(&self.city_root, site.building.addr().clone())?;

        let neighbours_tool = city::NeighboursTool::new(seen)?;

        let bound = self.read_bound(site);
        let candidates = nested_rule_candidates(site, addr, Arc::clone(&bound));
        let read_clock = Arc::clone(&self.clock);
        let read = runtime::ReadTool::new(
            &site.write_root,
            Arc::clone(&catalog),
            Arc::clone(&bound),
            &kernel::layout::CityLayout::new(&self.city_root).cas(),
        )?
        .storing_pictures(site.block_origin())
        .ruling(candidates.clone())
        .recalling(shown.clone())
        .clocking(Arc::new(move || read_clock.now()))
        .noting(stamps);
        let claim_tool = self.plan_tool(site, desks, &bound)?;

        let search = SearchTool::new(&site.write_root, Arc::clone(&bound))?;

        let succession = Arc::new(Mutex::new(runtime::SuccessionDesk::new()));
        let succeed = runtime::SucceedTool::new(Arc::clone(&succession))?;

        let domain = site.rules.write_domain()?;
        let scope: Vec<String> = domain.prefixes().map(|p| p.as_str().to_owned()).collect();
        let of = site.provenance(self.city, addr);
        let mut bench = ToolBench::new(domain)
            .with_checkpoint(runtime::bench::CheckpointNet {
                checkpoint: self.open_checkpoint(
                    &site.write_root,
                    &of,
                    site.rules.checkpoint_entropy_allow().clone(),
                )?,
                scope,
                of,
            })
            .for_job(addr.clone(), job_locator.clone());
        if let Some(listed) = site.rules.egress() {
            bench = bench.limit_egress(listed.clone());
        }
        *bench.taint_mut() = at.taint.clone();

        let own =
            |tool: Box<dyn kernel::Tool>| (Namespace::Own(ReservedNamespace::Sprawling), tool);

        let mut admitted: Vec<Filed> = vec![
            own(Box::new(archive_tool)),
            own(Box::new(claim_tool)),
            own(Box::new(edit)),
            own(Box::new(status)),
            own(Box::new(signal_tool)),
            own(Box::new(goal_tool)),
            own(Box::new(pr_tool)),
            own(Box::new(rules_tool)),
            own(Box::new(neighbours_tool)),
            own(Box::new(read)),
            own(Box::new(search)),
            own(Box::new(succeed)),
            own(Box::new(runtime::ToolTool::new(Arc::clone(&catalog))?)),
        ];
        self.push_vocation_tools(
            &mut admitted,
            VocationParts {
                site,
                at,
                desks,
                policy: policy.reader(),
            },
        )?;
        admitted.extend(self.filed_beyond_sprawling(site, addr, &bound)?);
        held(&catalog, "lay out the catalog")?.set_exposure(site.config.catalog.clone());
        register(&catalog, &mut bench, admitted, &keeper)?;
        self.admit_reading_room(&catalog, site, addr)?;
        Ok(Workbench {
            catalog,
            drift: self.take_drift()?,
            bench: Some(bench),
            policy: Some(policy),
            succession,
            context,
            shown,
            rule_candidates: candidates,
        })
    }

    fn push_vocation_tools(
        &self,
        admitted: &mut Vec<Filed>,
        parts: VocationParts<'_>,
    ) -> Result<(), AxError> {
        let own =
            |tool: Box<dyn kernel::Tool>| (Namespace::Own(ReservedNamespace::Sprawling), tool);
        match city::vocation_of(parts.site.building.addr()) {
            city::Vocation::Builds => {
                admitted.push(own(Box::new(self.exec_tool(
                    parts.site,
                    parts.at,
                    parts.policy,
                )?)));
                admitted.push(own(Box::new(collab::DelegateTool::new(Arc::clone(
                    &parts.desks.delegates,
                ))?)));
                admitted.push(own(Box::new(collab::WorkshopTool::new(
                    Arc::clone(&parts.desks.workshop),
                    Arc::clone(&parts.desks.delegates),
                )?)));
            }
            city::Vocation::Plans => {
                admitted.push(own(Box::new(city::CityTool::new(&self.city_root)?)));
            }
        }
        Ok(())
    }

    fn edit_tool(
        &self,
        site: &Site,
        addr: &Address,
        policy: runtime::PolicyReader,
        stamps: runtime::ReadStamps,
    ) -> Result<EditTool, AxError> {
        EditTool::new(
            &site.write_root,
            addr.clone(),
            site.rules.write_domain()?,
            runtime::EditDesk {
                policy,
                seen: stamps,
                locks: self.edits.clone(),
            },
        )
    }

    fn plan_tool(
        &self,
        site: &Site,
        desks: &BenchDesks,
        bound: &runtime::ReadBound,
    ) -> Result<collab::ClaimTool, AxError> {
        let pins = runtime::BoundReader::new(
            &site.write_root,
            Arc::clone(bound),
            &kernel::layout::CityLayout::new(&self.city_root).cas(),
        );
        Ok(
            collab::ClaimTool::new(Arc::clone(&desks.plan))?.pinning(collab::Pinner::new(
                move |asked| pins.pin(asked, "finish a plan node"),
            )),
        )
    }
}

type Filed = (Namespace, Box<dyn kernel::Tool>);

struct VocationParts<'a> {
    site: &'a Site,
    at: &'a Assignment,
    desks: &'a BenchDesks,
    policy: runtime::PolicyReader,
}

fn register(
    catalog: &Mutex<runtime::Catalog>,
    bench: &mut ToolBench,
    built: Vec<Filed>,
    keeper: &Arc<kept::Keeper>,
) -> Result<(), AxError> {
    for (namespace, tool) in built {
        let exposure = held(catalog, "lay out the catalog")?.admit_tool(tool.meta(), namespace)?;
        if exposure != kernel::config::Exposure::Off {
            bench.register(Box::new(kept::Kept::new(tool, Arc::clone(keeper))))?;
        }
    }
    Ok(())
}

impl Laying {
    fn filed_beyond_sprawling(
        &self,
        site: &Site,
        addr: &Address,
        bound: &runtime::ReadBound,
    ) -> Result<Vec<Filed>, AxError> {
        let mut filed: Vec<Filed> = Vec::new();

        let media = Namespace::Own(ReservedNamespace::Media);
        filed.extend(
            self.endpoint_tools(site, bound)?
                .into_iter()
                .map(|tool| (media.clone(), tool)),
        );

        let record = Namespace::Own(ReservedNamespace::Record);
        filed.push((record.clone(), Box::new(self.playback_tool(site, addr)?)));
        filed.push((record, Box::new(self.proposal_tool(site, addr, bound)?)));

        if let Some(search) = self.search_tool(site)? {
            filed.push((Namespace::Own(ReservedNamespace::Web), Box::new(search)));
        }
        filed.extend(
            self.outside_tools(site, addr)?
                .into_iter()
                .map(|tool| (outside_namespace(tool.meta()), tool)),
        );
        Ok(filed)
    }
}

fn outside_namespace(meta: &kernel::ToolMeta) -> Namespace {
    if let Effect::Connector { label } = &meta.effect {
        Namespace::Server(label.clone())
    } else {
        Namespace::Own(ReservedNamespace::Computer)
    }
}

impl Laying {
    fn outside_tools(
        &self,
        site: &Site,
        addr: &Address,
    ) -> Result<Vec<Box<dyn kernel::Tool>>, AxError> {
        let mut outside = (self.browsers)(&self.city_root, &site.block_origin(), &site.rules)?;

        let servers = self.servers(site);
        for server in self.mcp_tools(ConnectorJob {
            servers: &servers,
            references: &site.config.mcp_references,
            building: site.building.addr(),
            session: addr,
            write_root: &site.write_root,
            confidential: site.rules.policy().confidential,
        }) {
            let tool: Box<dyn kernel::Tool> = Box::new(server);
            outside.push(tool);
        }
        Ok(outside)
    }

    fn read_bound(&self, site: &Site) -> runtime::ReadBound {
        crate::prompt_chain::read_bound(&self.city_root, site.building.addr().clone())
    }

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
        let catalog = Arc::clone(reach.catalog);
        let read_catalog = move || match catalog.lock() {
            Ok(catalog) => match catalog.tool_table() {
                Ok(table) => Some(table.record),
                Err(_unreadable) => None,
            },
            Err(_poisoned) => None,
        };

        Ok(tool
            .reporting(self.backlog.clone())
            .metering(reach.context.clone())
            .clocked(site.clock.clone())
            .cataloging(Box::new(read_catalog)))
    }
}

pub(crate) fn catalog_builtins(
    search: Option<&kernel::config::SearchSupplier>,
) -> Result<Vec<(&'static str, kernel::ToolDef)>, kernel::AxError> {
    let record = kernel::ReservedNamespace::Record.as_str();
    let mut tools = vec![
        (record, playback::PlaybackTool::definition()?),
        (
            record,
            proposal::ProposalTool::<crate::worker::Relay>::definition()?,
        ),
    ];
    if let Some(supplier) = search {
        tools.push((
            kernel::ReservedNamespace::Web.as_str(),
            search::WebSearch::definition(supplier)?,
        ));
    }
    Ok(tools)
}
