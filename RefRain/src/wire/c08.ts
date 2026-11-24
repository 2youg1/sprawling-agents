// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  AgentCatalogAnswer,
  AgentOffer,
  ApprovalsAnswer,
  ArchiveAnswer,
  AutomationAnswer,
  BuildingAnswer,
  BytesAnswer,
} from "./c00";

import {
  CatalogAnswer,
  ChangesAnswer,
  CityAnswer,
  CommitAnswer,
  CommitsAnswer,
  ConfigAnswer,
  ContentAnswer,
  CostAnswer,
  CostOfAnswer,
  DevicesAnswer,
  RunSummary,
  Seq,
} from "./c01";

import {
  Ceiling,
  DiscardAnswer,
  DoctorAnswer,
  DoctorUpstream,
  DocumentAnswer,
  ModelTag,
} from "./c02";

import {
  EndpointsAnswer,
  EvidenceAnswer,
  ExportAnswer,
  FindAnswer,
  GitStatusAnswer,
  GithubLoginAnswer,
  GovernanceAnswer,
  GuideProgress,
  HistoryAnswer,
  HistoryRangeAnswer,
  HunksAnswer,
  IdentityAnswer,
  InboxAnswer,
  KnownHostsAnswer,
  Window,
} from "./c03";

import {
  AxError,
  Laid,
  LayersAnswer,
  ListingAnswer,
  McpHealthAnswer,
  McpUsageAnswer,
  MetricsAnswer,
  ModelFiguresAnswer,
  OpenProposalsAnswer,
} from "./c04";

import {
  PreferencesAnswer,
  PrefixAnswer,
  PreviewAnswer,
} from "./c05";

import {
  PrivacyAnswer,
  PromptShapeAnswer,
  ProposalsAnswer,
  RangeAnswer,
  RegistryAnswer,
  ReleaseAnswer,
  RoundsAnswer,
  RunCostsAnswer,
} from "./c06";

import {
  SessionsAnswer,
  ShellsAnswer,
  SkillUsageAnswer,
  SkillsAnswer,
  TemplateAnswer,
  ToolkitsAnswer,
  UsageExportAnswer,
  VersionsAnswer,
} from "./c07";

export const Answer = Schema.Union([
  Schema.Struct({
    history: HistoryAnswer,
  }),
  Schema.Struct({
    history_range: HistoryRangeAnswer,
  }),
  Schema.Struct({
    sessions: SessionsAnswer,
  }),
  Schema.Struct({
    changes: ChangesAnswer,
  }),
  Schema.Struct({
    commit: CommitAnswer,
  }),
  Schema.Struct({
    city: CityAnswer,
  }),
  Schema.Struct({
    run: Schema.NullOr(RunSummary),
  }),
  Schema.Struct({
    approvals: ApprovalsAnswer,
  }),
  Schema.Struct({
    cost: CostAnswer,
  }),
  Schema.Struct({
    endpoints: EndpointsAnswer,
  }),
  Schema.Struct({
    known_hosts: KnownHostsAnswer,
  }),
  Schema.Struct({
    agent_catalog: AgentCatalogAnswer,
  }),
  Schema.Struct({
    agent_spec: AgentOffer,
  }),
  Schema.Struct({
    devices: DevicesAnswer,
  }),
  Schema.Struct({
    building: BuildingAnswer,
  }),
  Schema.Struct({
    inbox: InboxAnswer,
  }),
  Schema.Struct({
    discards: DiscardAnswer,
  }),
  Schema.Struct({
    registry: RegistryAnswer,
  }),
  Schema.Struct({
    archive: ArchiveAnswer,
  }),
  Schema.Struct({
    metrics: MetricsAnswer,
  }),
  Schema.Struct({
    governance: GovernanceAnswer,
  }),
  Schema.Struct({
    hunks: HunksAnswer,
  }),
  Schema.Struct({
    rounds: RoundsAnswer,
  }),
  Schema.Struct({
    evidence: EvidenceAnswer,
  }),
  Schema.Struct({
    cost_of: CostOfAnswer,
  }),
  Schema.Struct({
    run_costs: RunCostsAnswer,
  }),
  Schema.Struct({
    listing: ListingAnswer,
  }),
  Schema.Struct({
    find: FindAnswer,
  }),
  Schema.Struct({
    document: DocumentAnswer,
  }),
  Schema.Struct({
    proposals: ProposalsAnswer,
  }),
  Schema.Struct({
    open_proposals: OpenProposalsAnswer,
  }),
  Schema.Struct({
    range: RangeAnswer,
  }),
  Schema.Struct({
    versions: VersionsAnswer,
  }),
  Schema.Struct({
    bytes: BytesAnswer,
  }),
  Schema.Struct({
    export: ExportAnswer,
  }),
  Schema.Struct({
    preview: PreviewAnswer,
  }),
  Schema.Struct({
    reply: Laid,
  }),
  Schema.Struct({
    commits: CommitsAnswer,
  }),
  Schema.Struct({
    doctor: DoctorAnswer,
  }),
  Schema.Struct({
    upstream: DoctorUpstream,
  }),
  Schema.Struct({
    prefix: PrefixAnswer,
  }),
  Schema.Struct({
    prompt_shape: PromptShapeAnswer,
  }),
  Schema.Struct({
    content: ContentAnswer,
  }),
  Schema.Struct({
    skills: SkillsAnswer,
  }),
  Schema.Struct({
    template: TemplateAnswer,
  }),
  Schema.Struct({
    layers: LayersAnswer,
  }),
  Schema.Struct({
    catalog: CatalogAnswer,
  }),
  Schema.Struct({
    git_status: GitStatusAnswer,
  }),
  Schema.Struct({
    mcp_health: McpHealthAnswer,
  }),
  Schema.Struct({
    skill_usage: SkillUsageAnswer,
  }),
  Schema.Struct({
    mcp_usage: McpUsageAnswer,
  }),
  Schema.Struct({
    usage_export: UsageExportAnswer,
  }),
  Schema.Struct({
    shells: ShellsAnswer,
  }),
  Schema.Struct({
    toolkits: ToolkitsAnswer,
  }),
  Schema.Struct({
    release: ReleaseAnswer,
  }),
  Schema.Struct({
    preferences: PreferencesAnswer,
  }),
  Schema.Struct({
    config: ConfigAnswer,
  }),
  Schema.Struct({
    identity: IdentityAnswer,
  }),
  Schema.Struct({
    automation: AutomationAnswer,
  }),
  Schema.Struct({
    github_login: GithubLoginAnswer,
  }),
  Schema.Struct({
    guide: GuideProgress,
  }),
  Schema.Struct({
    privacy: PrivacyAnswer,
  }),
  Schema.Struct({
    model_figures: ModelFiguresAnswer,
  }),
  Schema.Struct({
    unavailable: Schema.Struct({
      query: Schema.String,
      reason: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
]).annotate({ identifier: "Answer" });
export type Answer = typeof Answer.Type;

export const AskId = Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)).pipe(Schema.brand("AskId"));
export type AskId = typeof AskId.Type;

export const AskOutcome = Schema.Union([
  Schema.Struct({
    answer: Answer,
  }),
  Schema.Struct({
    refusal: AxError,
  }),
]).annotate({ identifier: "AskOutcome" });
export type AskOutcome = typeof AskOutcome.Type;

export const Answered = Schema.Struct({
  as_of: Seq,
  ask_id: AskId,
  outcome: AskOutcome,
}).annotate({ identifier: "Answered" });
export type Answered = typeof Answered.Type;

export const FiguresAsked = Schema.Struct({
  context_tokens: Schema.optional(Schema.NullOr(Window)),
  id: Schema.String,
  max_output_tokens: Schema.optional(Schema.NullOr(Ceiling)),
  tag: Schema.optional(Schema.NullOr(ModelTag)),
}).annotate({ identifier: "FiguresAsked" });
export type FiguresAsked = typeof FiguresAsked.Type;

export const ProviderName = Schema.String.pipe(Schema.brand("ProviderName"));
export type ProviderName = typeof ProviderName.Type;

export const ReplyState = Schema.Literals(["streaming", "settled"]).annotate({ identifier: "ReplyState" });
export type ReplyState = typeof ReplyState.Type;

