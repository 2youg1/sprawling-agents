// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, B3Hash, DialectKind, GitOid, Locator, NodeId, RunId, Seq};
use serde::{Deserialize, Serialize};

use crate::named_frames::named_frames;

named_frames! {

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Query {

    History {
        before: Option<Seq>,
        limit: u32,
    },

    RunHistory {
        run: RunId,
        before: Option<Seq>,
        limit: u32,
    },

    HistoryRange {
        from: Seq,
        to: Seq,
        limit: u32,
    },

    Sessions {
        room: Address,
    },

    Changes {
        base: GitOid,
        head: Option<GitOid>,
    },

    Hunks {
        oid_a: GitOid,
        oid_b: GitOid,
        path: String,
    },

    Commit {
        oid: GitOid,
    },
    RunView {
        run: RunId,
    },
    CityView,
    ApprovalQueue,
    InboxView {
        addr: Address,
    },
    Metrics,
    CostView,
    ArchiveSearch {
        needle: String,
    },
    RegistryView,
    DiscardView,

    EndpointView,

    KnownHosts,

    AgentCatalog,

    ParseAgentSpec { text: String },

    Devices,

    BuildingView {
        addr: Address,
    },

    Identity,

    Automation,

    GithubLogin(Option<String>),

    Guide,

    Governance,

    Rounds {
        run: RunId,
    },

    Evidence {
        run: RunId,
    },

    CostOf {
        node: NodeId,
    },

    Listing {
        at: Option<Address>,
    },

    Find { under: Address, text: String },

    Document {
        at: Address,
    },

    Proposals(Address),

    OpenProposals,

    Range { version: B3Hash, range: documents::Span },

    Versions { at: Address },

    Bytes { version: B3Hash, range: documents::Span },

    Export { at: Address, version: B3Hash },

    Preview { version: B3Hash, viewport: documents::Span },

    Reply {
        text: String,
        state: documents::ReplyState,
    },

    Commits {
        building: Option<Address>,
        before: Option<Seq>,
        limit: u32,
    },

    Doctor,

    Prefix {
        run: RunId,
    },

    PromptShape {
        run: RunId,
    },

    Content {
        locator: Locator,
    },

    Skills { building: Address },

    Template {
        template: crate::TemplateName,
        addr: Address,
    },

    GitStatus { building: Address },

    McpHealth {
        addr: Address,
    },

    Toolkits,

    NewestRelease,

    Privacy,

    Preferences,

    Config {
        addr: Address,
    },

    RunCosts {
        runs: Vec<RunId>,
    },

    UpstreamVersion {
        item: String,
    },

    SkillUsage { skill: Option<String> },

    McpUsage { server: Option<String> },

    UsageExport { what: crate::UsageKind, format: crate::ExportFormat },

    Shells,

    ModelFigures {
        endpoint: crate::ProviderName,
        base_url: String,
        face: DialectKind,
        models: Vec<crate::FiguresAsked>,
    },

    Layers { addr: Address },

    Catalog { building: Address },
}

pub const QUERY_NAMES;
}
