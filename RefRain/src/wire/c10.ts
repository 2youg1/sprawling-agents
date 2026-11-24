// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  Address,
  AgentAdding,
  AgentLoginStart,
  ApprovalId,
  B3Hash,
  IdemKey,
  Span,
} from "./c00";

import {
  Effort,
  GitOid,
  HaltScope,
  Restoration,
  RunId,
  Seq,
  SessionName,
} from "./c01";

import {
  Ceiling,
  DialectKind,
  ModelTag,
} from "./c02";

import {
  Autonomy,
  EndpointTuning,
  GovernedDocument,
  GuideProgress,
  ResidentId,
  Ruling,
  Window,
} from "./c03";

import {
  InputKinds,
} from "./c04";

import {
  Carry,
  Origin,
  RunPolicy,
} from "./c06";

import {
  TemplateName,
  ToolkitSlug,
} from "./c07";

import {
  ProviderName,
} from "./c08";

import {
  BeatMs,
  BuildingSettings,
  CityClosing,
  CitySettings,
  DeviceForgetting,
  DoorAnswer,
  DoorOpening,
  DoorStep,
  IdentityCard,
  NoSecret,
  PolicyChange,
  PreferencePatch,
  PrivacyRequest,
  PromptWrite,
  Verdict,
} from "./c09";

export const SliceVerdict = Schema.Struct({
  slice: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  verdict: Verdict,
}).annotate({ identifier: "SliceVerdict" });
export type SliceVerdict = typeof SliceVerdict.Type;

export const ProposalDecision = Schema.Struct({
  proposal: B3Hash,
  verdicts: Schema.Array(SliceVerdict),
}).annotate({ identifier: "ProposalDecision" });
export type ProposalDecision = typeof ProposalDecision.Type;

export const ProposalDecisions = Schema.Struct({
  decisions: Schema.Array(ProposalDecision),
  doc: Address,
  idem: IdemKey,
}).annotate({ identifier: "ProposalDecisions" });
export type ProposalDecisions = typeof ProposalDecisions.Type;

export const PursuitStep = Schema.Union([
  Schema.Literals(["pause", "resume", "clear"]),
  Schema.Struct({
    set: Schema.Struct({
      goal: Schema.String,
    }),
  }),
]).annotate({ identifier: "PursuitStep" });
export type PursuitStep = typeof PursuitStep.Type;

export const TextEdit = Schema.Struct({
  span: Span,
  text: Schema.String,
}).annotate({ identifier: "TextEdit" });
export type TextEdit = typeof TextEdit.Type;

export const RangeWrite = Schema.Struct({
  baseline: B3Hash,
  doc: Address,
  edits: Schema.Array(TextEdit),
  idem: IdemKey,
}).annotate({ identifier: "RangeWrite" });
export type RangeWrite = typeof RangeWrite.Type;

export const RulesWrite = Schema.Struct({
  base: Schema.String,
  body: Schema.String,
  building: Address,
  idem: IdemKey,
}).annotate({ identifier: "RulesWrite" });
export type RulesWrite = typeof RulesWrite.Type;

export const SecretForgetting = Schema.Struct({
  idem: IdemKey,
  reference: Schema.String,
}).annotate({ identifier: "SecretForgetting" });
export type SecretForgetting = typeof SecretForgetting.Type;

export const SessionNaming = Schema.Struct({
  began: Seq,
  idem: IdemKey,
  name: Schema.String,
  room: Address,
}).annotate({ identifier: "SessionNaming" });
export type SessionNaming = typeof SessionNaming.Type;

export const Shelf = Schema.Union([
  Schema.Literal("library"),
  Schema.Struct({
    building: Address,
  }),
]).annotate({ identifier: "Shelf" });
export type Shelf = typeof Shelf.Type;

export const SpineDocument = Schema.Literals(["roadmap", "memo", "handoff", "spec"]).annotate({ identifier: "SpineDocument" });
export type SpineDocument = typeof SpineDocument.Type;

export const WorkspaceChoiceWrite = Schema.Struct({
  base: Schema.String,
  building: Address,
  effort: Effort,
  endpoint: Schema.String,
  idem: IdemKey,
  model: Schema.String,
}).annotate({ identifier: "WorkspaceChoiceWrite" });
export type WorkspaceChoiceWrite = typeof WorkspaceChoiceWrite.Type;

export const Command = Schema.Union([
  Schema.Struct({
    dispatch: Schema.Struct({
      addr: Address,
      effort: Schema.optional(Schema.NullOr(Effort)),
      goal: Schema.String,
      idem: IdemKey,
      images: Schema.optional(Schema.Array(Schema.String)),
      model: Schema.optional(Schema.NullOr(Schema.String)),
      policy: RunPolicy,
      resume: Schema.optional(Schema.NullOr(Seq)),
      session: Schema.optional(Schema.NullOr(SessionName)),
      task: Schema.String,
    }),
  }),
  Schema.Struct({
    probe_endpoint: Schema.Struct({
      auth_header: Schema.optional(Schema.NullOr(Schema.String)),
      base_url: Schema.String,
      dialect: DialectKind,
      idem: IdemKey,
      name: ProviderName,
      secret: Schema.optional(Schema.NullOr(Schema.String)),
      tuning: EndpointTuning,
    }),
  }),
  Schema.Struct({
    configure_building: BuildingSettings,
  }),
  Schema.Struct({
    attach_endpoint: Schema.Struct({
      admit: Schema.Array(Schema.String),
      auth_header: Schema.optional(Schema.NullOr(Schema.String)),
      base_url: Schema.String,
      dialect: DialectKind,
      idem: IdemKey,
      name: ProviderName,
      named: Schema.Array(Schema.String),
      secret: Schema.optional(Schema.NullOr(Schema.String)),
      tuning: EndpointTuning,
    }),
  }),
  Schema.Struct({
    select_model: Schema.Struct({
      context_tokens: Schema.optional(Schema.NullOr(Window)),
      endpoint: ProviderName,
      idem: IdemKey,
      input: Schema.optional(Schema.NullOr(InputKinds)),
      max_output_tokens: Schema.optional(Schema.NullOr(Ceiling)),
      model: Schema.String,
      tag: ModelTag,
    }),
  }),
  Schema.Struct({
    open_session: Schema.Struct({
      addr: Address,
      carry: Carry,
      from: Schema.optional(Schema.NullOr(Origin)),
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    create_building: Schema.Struct({
      addr: Address,
      idem: IdemKey,
      rules: Schema.optional(Schema.NullOr(Schema.String)),
      source: Schema.optional(Schema.NullOr(Schema.String)),
      template: TemplateName,
    }),
  }),
  Schema.Struct({
    remove_building: Schema.Struct({
      addr: Address,
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    put_secret: Schema.Struct({
      name: Schema.String,
      realm: Schema.String,
      value: NoSecret,
    }),
  }),
  Schema.Struct({
    steer: Schema.Struct({
      idem: IdemKey,
      images: Schema.optional(Schema.Array(Schema.String)),
      run: RunId,
      text: Schema.String,
    }),
  }),
  Schema.Struct({
    cancel: Schema.Struct({
      idem: IdemKey,
      run: RunId,
    }),
  }),
  Schema.Struct({
    halt: Schema.Struct({
      idem: IdemKey,
      scope: HaltScope,
    }),
  }),
  Schema.Struct({
    reveal: Schema.Struct({
      at: Address,
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    restore_discard: Schema.Struct({
      idem: IdemKey,
      restoration: Restoration,
    }),
  }),
  Schema.Struct({
    doctor_install: Schema.Struct({
      idem: IdemKey,
      item: Schema.String,
    }),
  }),
  Schema.Struct({
    doctor_refresh: Schema.Struct({
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    release: Schema.Struct({
      idem: IdemKey,
      scope: HaltScope,
    }),
  }),
  Schema.Struct({
    batch_by_building: Schema.Struct({
      addr: Address,
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    approve: Schema.Struct({
      idem: IdemKey,
      item: ApprovalId,
      verdict: Ruling,
    }),
  }),
  Schema.Struct({
    hand_off: Schema.Struct({
      idem: IdemKey,
      item: ApprovalId,
      to: ResidentId,
    }),
  }),
  Schema.Struct({
    set_autonomy: Schema.Struct({
      autonomy: Autonomy,
      idem: IdemKey,
      scope: HaltScope,
    }),
  }),
  Schema.Struct({
    pursue: Schema.Struct({
      addr: Address,
      idem: IdemKey,
      step: PursuitStep,
    }),
  }),
  Schema.Struct({
    wake: Schema.Struct({
      body: Schema.String,
      idem: IdemKey,
      source: Schema.String,
      subject: Schema.String,
    }),
  }),
  Schema.Struct({
    put_document: Schema.Struct({
      base: Schema.String,
      body: Schema.String,
      idem: IdemKey,
      which: GovernedDocument,
    }),
  }),
  Schema.Struct({
    put_identity: Schema.Struct({
      base: Schema.String,
      card: IdentityCard,
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    put_rules: RulesWrite,
  }),
  Schema.Struct({
    set_workspace_choice: WorkspaceChoiceWrite,
  }),
  Schema.Struct({
    restore_file: Schema.Struct({
      at: Address,
      idem: IdemKey,
      point: GitOid,
    }),
  }),
  Schema.Struct({
    put_guide: Schema.Struct({
      idem: IdemKey,
      progress: GuideProgress,
    }),
  }),
  Schema.Struct({
    configure_city: CitySettings,
  }),
  Schema.Struct({
    put_spine: Schema.Struct({
      base: Schema.String,
      body: Schema.String,
      building: Address,
      idem: IdemKey,
      which: SpineDocument,
    }),
  }),
  Schema.Struct({
    put_range: RangeWrite,
  }),
  Schema.Struct({
    decide_proposals: ProposalDecisions,
  }),
  Schema.Struct({
    connect_toolkit: Schema.Struct({
      idem: IdemKey,
      toolkit: ToolkitSlug,
    }),
  }),
  Schema.Struct({
    put_preferences: Schema.Struct({
      idem: IdemKey,
      patch: PreferencePatch,
    }),
  }),
  Schema.Struct({
    put_shelved: Schema.Struct({
      idem: IdemKey,
      name: Schema.String,
      shelf: Shelf,
      text: Schema.String,
    }),
  }),
  Schema.Struct({
    name_session: SessionNaming,
  }),
  Schema.Struct({
    change_run_policy: PolicyChange,
  }),
  Schema.Struct({
    open_remote_door: DoorOpening,
  }),
  Schema.Struct({
    replace_city_key: DoorStep,
  }),
  Schema.Struct({
    confirm_remote_door: DoorAnswer,
  }),
  Schema.Struct({
    close_remote_door: DoorStep,
  }),
  Schema.Struct({
    privacy_operation: PrivacyRequest,
  }),
  Schema.Struct({
    forget_secret: SecretForgetting,
  }),
  Schema.Struct({
    close_city: CityClosing,
  }),
  Schema.Struct({
    add_agent: AgentAdding,
  }),
  Schema.Struct({
    agent_login: AgentLoginStart,
  }),
  Schema.Struct({
    forget_device: DeviceForgetting,
  }),
  Schema.Struct({
    put_prompt: PromptWrite,
  }),
  Schema.Struct({
    auth: Schema.Struct({
      token: Schema.String,
    }),
  }),
]).annotate({ identifier: "Command" });
export type Command = typeof Command.Type;

export const Hello = Schema.Struct({
  schema: B3Hash,
  token: Schema.optional(Schema.NullOr(Schema.String)),
  wire_v: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Hello" });
export type Hello = typeof Hello.Type;

export const Monitoring = Schema.Union([
  Schema.Literals(["watch", "watch_summary", "release"]),
  Schema.Struct({
    beat: BeatMs,
  }),
]).annotate({ identifier: "Monitoring" });
export type Monitoring = typeof Monitoring.Type;

