// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The four faces of one building `ConfigureBuilding` carries - its MCP
// servers, its desktop allowlist, its context rung and its sandbox -
// one builder each, apart from `commands.ts` for the length budget that
// file sits at. Each builder fills its own face and writes `null` into
// the other three, because the city reads `null` as "leave this face as
// it is" (`assembly/commanding/configure.rs`).

import { mintIdem } from "../idem";
import type { Address, Command, McpServer, SandboxLimits } from "../../wire";

export function configureMcp(addr: Address, mcp: readonly McpServer[]): Command {
  return {
    configure_building: {
      addr,
      mcp: [...mcp],
      sandbox: null,
      desktop: null,
      context_second_threshold: null,
      idem: mintIdem(),
    },
  };
}

// The windows on this person's own machine a building's connector may
// touch. Sent as text, because the connector that reads the file is the
// authority on its syntax and this page must not become a second one.
export function configureDesktop(addr: Address, allowlist: string): Command {
  return {
    configure_building: {
      addr,
      mcp: null,
      sandbox: null,
      desktop: allowlist,
      context_second_threshold: null,
      idem: mintIdem(),
    },
  };
}

// Where the context reminder's second rung sits: a whole percent of the
// window. Sent as a raw number because the domain is
// `kernel::config::SecondThreshold`'s one construction point - a form
// that enforced it here would be the second place that rule lives - and
// the refusal comes back carrying the legal span.
export function configureContext(addr: Address, percent: number): Command {
  return {
    configure_building: {
      addr,
      mcp: null,
      sandbox: null,
      desktop: null,
      context_second_threshold: percent,
      idem: mintIdem(),
    },
  };
}

// A building's sandbox, whole: the city resolves the sandbox as one
// value, so a layer that speaks about it speaks about all of it
// (`kernel::config::SandboxLimits`), and the card sends what it shows.
export function configureSandbox(addr: Address, limits: SandboxLimits): Command {
  return {
    configure_building: {
      addr,
      mcp: null,
      sandbox: limits,
      desktop: null,
      context_second_threshold: null,
      idem: mintIdem(),
    },
  };
}
