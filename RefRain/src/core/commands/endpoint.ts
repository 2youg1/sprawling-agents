// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type {
  AccountRetries,
  Command,
  DialectKind,
  EndpointSummary,
  EndpointTuning,
  ProviderAccount,
  ProviderName,
  Proxying,
} from "../../wire";
import type { BodyOverride, ModelGeneration } from "../../wire";
import { ProviderName as ProviderNameSchema } from "../../wire";
import { mintIdem } from "../idem";

export interface Endpoint {
  readonly id: string;
  readonly baseUrl: string;
  readonly dialect: DialectKind;
  readonly secret: string | null;
  readonly authHeader: string | null;
  readonly tuning: Tuning;
}

export interface Pair {
  readonly name: string;
  readonly value: string;
}

export interface Tuning {
  readonly label: string | null;
  readonly timeoutMs: number | null;
  readonly requestMaxRetries: number | null;
  readonly streamIdleTimeoutMs: number | null;
  readonly headers: readonly Pair[];
  readonly overrides: readonly Pair[];
  readonly proxying: Proxying | null;
}

function count(stated: number | null): number | null {
  return stated !== null && Number.isInteger(stated) && stated >= 0 ? stated : null;
}

function span(stated: number | null): number | null {
  const whole = count(stated);
  return whole !== null && whole > 0 ? whole : null;
}

function tuningFrame(tuning: Tuning): EndpointTuning {
  const named = (rows: readonly Pair[]) => rows.filter((row) => row.name.trim() !== "");
  return {
    label: tuning.label === null || tuning.label.trim() === "" ? null : tuning.label.trim(),
    timeout_ms: span(tuning.timeoutMs),
    request_max_retries: count(tuning.requestMaxRetries),
    stream_idle_timeout_ms: span(tuning.streamIdleTimeoutMs),
    headers: named(tuning.headers).map((row) => ({ name: row.name.trim(), value: row.value })),
    overrides: named(tuning.overrides).map((row) => ({
      pointer: row.name.trim(),
      value: row.value,
    })),
    proxying: tuning.proxying,
  };
}

export function providerName(name: string): ProviderName {
  return ProviderNameSchema.make(name);
}

export function probeEndpoint(e: Endpoint): Command {
  return {
    probe_endpoint: {
      name: providerName(e.id),
      base_url: e.baseUrl,
      dialect: e.dialect,
      secret: e.secret,
      auth_header: e.authHeader,
      tuning: tuningFrame(e.tuning),
      idem: mintIdem(),
    },
  };
}

export function attachEndpoint(
  e: Endpoint,
  admit: readonly string[],
  named: readonly string[],
): Command {
  return {
    attach_endpoint: {
      name: providerName(e.id),
      base_url: e.baseUrl,
      dialect: e.dialect,
      secret: e.secret,
      auth_header: e.authHeader,
      admit: [...admit],
      named: [...named],
      tuning: tuningFrame(e.tuning),
      idem: mintIdem(),
    },
  };
}

export interface Admission {
  readonly listed: readonly string[];
  readonly named: readonly string[];
}

export function probeAttached(summary: EndpointSummary): Command {
  return {
    probe_endpoint: {
      name: providerName(summary.name),
      base_url: summary.base_url,
      dialect: summary.dialect,
      secret: null,
      auth_header: null,
      tuning: { ...summary.tuning, accounts: null },
      idem: mintIdem(),
    },
  };
}

export function saveModels(
  summary: EndpointSummary,
  admit: Admission,
  generation: readonly ModelGeneration[] = summary.tuning.generation ?? [],
  overrides: readonly BodyOverride[] = summary.tuning.overrides,
): Command {
  return {
    attach_endpoint: {
      name: providerName(summary.name),
      base_url: summary.base_url,
      dialect: summary.dialect,
      secret: null,
      auth_header: null,
      admit: [...admit.listed],
      named: [...admit.named],
      tuning: {
        ...summary.tuning,
        accounts: null,
        generation: [...generation],
        overrides: [...overrides],
      },
      idem: mintIdem(),
    },
  };
}

export function reattachEndpoint(
  summary: EndpointSummary,
  accounts: readonly ProviderAccount[],
  retries: AccountRetries | null,
): Command {
  return {
    attach_endpoint: {
      name: providerName(summary.name),
      base_url: summary.base_url,
      dialect: summary.dialect,
      secret: null,
      auth_header: null,
      admit: summary.models.filter((model) => model.listing === "listed").map((model) => model.id),
      named: summary.models.filter((model) => model.listing === "named").map((model) => model.id),
      tuning: { ...summary.tuning, accounts: [...accounts], account_retries: retries },
      idem: mintIdem(),
    },
  };
}
