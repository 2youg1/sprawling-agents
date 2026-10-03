// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two tables the fold answers: one line per skill name, and one
//! entry per tool server with its tools (wire D33).

use std::collections::{BTreeMap, BTreeSet};

use super::{McpCall, Shelved, Usage, per_day};

impl Usage {
    /// One line per skill name that a shelf holds or a run read, in name
    /// order; `only` narrows it to one name.
    pub(crate) fn skills(&self, shelved: &[Shelved], only: Option<&str>) -> wire::SkillUsageAnswer {
        let names: BTreeSet<&str> = shelved
            .iter()
            .map(|copy| copy.name.as_str())
            .chain(self.versions.keys().map(String::as_str))
            .chain(self.reads.iter().map(|read| read.name.as_str()))
            .filter(|name| only.is_none_or(|wanted| wanted == *name))
            .collect();
        wire::SkillUsageAnswer {
            skills: names
                .into_iter()
                .map(|name| self.skill_line(name, shelved))
                .collect(),
        }
    }

    fn skill_line(&self, name: &str, shelved: &[Shelved]) -> wire::SkillUsageLine {
        let audits = self.audits.get(name).map_or(&[][..], Vec::as_slice);
        let reads: Vec<_> = self.reads.iter().filter(|read| read.name == name).collect();
        wire::SkillUsageLine {
            name: name.to_owned(),
            held: shelved
                .iter()
                .filter(|copy| copy.name == name)
                .map(|copy| wire::HeldSkill {
                    shelf: copy.shelf.clone(),
                    digest: copy.digest,
                    audit: audit_of(city::audit_state(&copy.digest, audits)),
                })
                .collect(),
            versions: self.versions.get(name).cloned().unwrap_or_default(),
            uses: reads
                .iter()
                .map(|read| wire::SkillUse {
                    run: read.used.run,
                    resident: read.used.resident.clone(),
                    seq: read.used.seq,
                    at: read.used.at,
                    part: read.part.clone(),
                    digest: read.digest,
                    outcome: self.outcome(&read.used),
                })
                .collect(),
            per_day: per_day(reads.iter().map(|read| read.used.at)),
        }
    }

    /// One entry per tool server that a building is configured with or a
    /// call recorded, in server order, with the calls no known server
    /// name prefixes gathered last under `None`; `only` narrows it to one
    /// server.
    pub(crate) fn mcp(
        &self,
        configured: &BTreeSet<String>,
        only: Option<&str>,
    ) -> wire::McpUsageAnswer {
        let known: BTreeSet<&str> = configured
            .iter()
            .map(String::as_str)
            .chain(self.calls.iter().filter_map(|call| call.label.as_deref()))
            .collect();
        let mut servers: BTreeMap<Option<&str>, BTreeMap<String, Vec<wire::McpUse>>> = known
            .iter()
            .map(|label| (Some(*label), BTreeMap::new()))
            .collect();
        for call in &self.calls {
            let (server, tool) = attributed(call, &known);
            servers
                .entry(server)
                .or_default()
                .entry(tool)
                .or_default()
                .push(wire::McpUse {
                    run: call.used.run,
                    resident: call.used.resident.clone(),
                    seq: call.used.seq,
                    at: call.used.at,
                    outcome: self.outcome(&call.used),
                });
        }
        let removed = servers.remove(&None).map(|tools| (None, tools));
        wire::McpUsageAnswer {
            servers: servers
                .into_iter()
                .chain(removed)
                .filter(|(server, _)| only.is_none_or(|wanted| *server == Some(wanted)))
                .map(|(server, tools)| wire::McpServerUsage {
                    configured: server.is_some_and(|label| configured.contains(label)),
                    server: server.map(str::to_owned),
                    tools: tools
                        .into_iter()
                        .map(|(tool, uses)| wire::McpToolUsage {
                            per_day: per_day(uses.iter().map(|used| used.at)),
                            tool,
                            uses,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// The server one call belongs to and its tool's name under it.
///
/// A call whose record named its server keeps it. A `call` line names
/// only the whole tool name: a server label holds no `_`
/// (`kernel::ServerLabel`), so the head before the first `_` is the
/// server when it is a known one, and otherwise the call is filed under
/// no server - the city's own tools hold `_` too (wire D33).
fn attributed<'a>(call: &'a McpCall, known: &BTreeSet<&'a str>) -> (Option<&'a str>, String) {
    if let Some(label) = call.label.as_deref() {
        return (Some(label), call.tool.clone());
    }
    call.tool
        .split_once('_')
        .and_then(|(head, tool)| known.get(head).map(|label| (Some(*label), tool.to_owned())))
        .unwrap_or_else(|| (None, call.tool.clone()))
}

/// The wire's spelling of what the audits say about one content.
fn audit_of(state: city::AuditState) -> wire::SkillAudit {
    match state {
        city::AuditState::Unaudited => wire::SkillAudit::Unaudited,
        city::AuditState::Audited { verdict, at } => wire::SkillAudit::Audited { verdict, at },
        city::AuditState::Stale { audited } => wire::SkillAudit::Stale { audited },
    }
}
