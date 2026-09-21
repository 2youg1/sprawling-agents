// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the settings page has to say to attach one endpoint, and the two
// frames it says it in. Separate from the rest of the commands because
// this is the only family with a shape of its own: every other verb is
// a sentence the page already holds, and this one is a form a person
// fills in over several screens before anything is sent.

import type {
  Command,
  DialectKind,
  EndpointTuning,
  ProviderName,
  Proxying,
} from "../../wire";
import { ProviderName as ProviderNameSchema } from "../../wire";
import { mintIdem } from "../idem";

export interface Endpoint {
  // `[model_providers.<id>]` in Codex's config.toml: the key the
  // credential reference is derived from, and the name the city files
  // the endpoint under.
  readonly id: string;
  readonly baseUrl: string;
  readonly dialect: DialectKind;
  readonly secret: string | null;
  readonly authHeader: string | null;
  readonly tuning: Tuning;
}

// One row of either key-value table the form draws: a header every
// request carries, or a JSON pointer into the body it sends.
export interface Pair {
  readonly name: string;
  readonly value: string;
}

// What a person settled about one endpoint besides its address, in the
// spelling Codex's `[model_providers.<id>]` uses. Every figure is
// absent until somebody states one, and the city reads an absent figure
// as its own default rather than as zero.
export interface Tuning {
  readonly label: string | null;
  readonly timeoutMs: number | null;
  readonly requestMaxRetries: number | null;
  readonly streamIdleTimeoutMs: number | null;
  readonly headers: readonly Pair[];
  readonly overrides: readonly Pair[];
  // Which of this endpoint's calls go through the machine's proxy. The
  // city's own rule keeps a call to an address on this machine off it,
  // and that is what `null` asks for.
  readonly proxying: Proxying | null;
}

// A figure reaches the wire only as a whole number that is not
// negative. A retry count of zero is a real answer - "once, then report"
// - so it travels, while every deadline is refused at zero: no request
// completes in no time, and a cleared box means "the city's own".
function count(stated: number | null): number | null {
  return stated !== null && Number.isInteger(stated) && stated >= 0 ? stated : null;
}

function span(stated: number | null): number | null {
  const whole = count(stated);
  return whole !== null && whole > 0 ? whole : null;
}

// The tuning as the frame carries it. A row whose name is blank is left
// out: the form keeps an empty row open while somebody types into it,
// and a half-written header must not reach a provider.
function tuningFrame(tuning: Tuning): EndpointTuning {
  const named = (rows: readonly Pair[]) => rows.filter((row) => row.name.trim() !== "");
  return {
    label: tuning.label === null || tuning.label.trim() === "" ? null : tuning.label.trim(),
    timeout_ms: span(tuning.timeoutMs),
    request_max_retries: count(tuning.requestMaxRetries),
    stream_idle_timeout_ms: span(tuning.streamIdleTimeoutMs),
    headers: named(tuning.headers).map((row) => ({ name: row.name.trim(), value: row.value })),
    overrides: named(tuning.overrides).map((row) => ({ pointer: row.name.trim(), value: row.value })),
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

export function attachEndpoint(e: Endpoint, admit: readonly string[]): Command {
  return {
    attach_endpoint: {
      name: providerName(e.id),
      base_url: e.baseUrl,
      dialect: e.dialect,
      secret: e.secret,
      auth_header: e.authHeader,
      admit: [...admit],
      tuning: tuningFrame(e.tuning),
      idem: mintIdem(),
    },
  };
}
