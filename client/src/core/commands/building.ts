// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The five faces of one building `ConfigureBuilding` carries - its MCP
// servers, its desktop allowlist, its context rung, its sandbox and its
// `[catalog]` table - one builder each, apart from `commands.ts` for the
// length budget that file sits at. Each builder fills its own face and
// writes `null` into the others, because the city reads `null` as "leave
// this face as it is" (`accounting/src/worker/commanding/configure.rs`).

import { mintIdem } from "../idem";
import type { Address, Command, ExposureTable, McpServer, SandboxLimits } from "../../wire";

export function configureMcp(addr: Address, mcp: readonly McpServer[]): Command {
  return {
    configure_building: {
      addr,
      mcp: [...mcp],
      sandbox: null,
      desktop: null,
      context_second_threshold: null,
      catalog: null,
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
      catalog: null,
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
      catalog: null,
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
      catalog: null,
      idem: mintIdem(),
    },
  };
}

// A building's whole `[catalog]` table: how each tool reaches its runs.
// It replaces what the building stated, and an empty table hands every
// tool back to the city's table (`crates/wire/Spec.lean` §8-99).
export function configureCatalog(addr: Address, catalog: ExposureTable): Command {
  return {
    configure_building: {
      addr,
      mcp: null,
      sandbox: null,
      desktop: null,
      context_second_threshold: null,
      catalog,
      idem: mintIdem(),
    },
  };
}
