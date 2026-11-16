// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The faces of one building `ConfigureBuilding` carries. Each builder
// fills its own face and writes `null` into the others, because the city
// reads `null` as "leave this face as it is".

import { mintIdem } from "../idem";
import type {
  Address,
  B3Hash,
  Command,
  ExposureTable,
  McpReference,
  McpServer,
  SandboxLimits,
} from "../../wire";

// The whole editable building catalog. References carry labels and pins,
// never another building's server definitions.
export interface CatalogSelection {
  readonly catalog: ExposureTable;
  readonly mcp_references: readonly McpReference[];
  readonly skill_pins: Readonly<Record<string, B3Hash>>;
  readonly reading_room: readonly string[];
}

function absent(addr: Address) {
  return {
    addr,
    mcp: null,
    sandbox: null,
    desktop: null,
    context_second_threshold: null,
    catalog: null,
    mcp_references: null,
    skill_pins: null,
    reading_room: null,
  };
}

export function configureMcp(addr: Address, mcp: readonly McpServer[]): Command {
  return { configure_building: { ...absent(addr), mcp: [...mcp], idem: mintIdem() } };
}

// The windows on this person's own machine a building's connector may
// touch. Sent as text, because the connector that reads the file is the
// authority on its syntax and this page must not become a second one.
export function configureDesktop(addr: Address, allowlist: string): Command {
  return { configure_building: { ...absent(addr), desktop: allowlist, idem: mintIdem() } };
}

// Where the context reminder's second rung sits: a whole percent of the
// window. Sent as a raw number because the domain is
// `kernel::config::SecondThreshold`'s one construction point.
export function configureContext(addr: Address, percent: number): Command {
  return {
    configure_building: { ...absent(addr), context_second_threshold: percent, idem: mintIdem() },
  };
}

// A building's sandbox, whole: the city resolves the sandbox as one
// value, so a layer that speaks about it speaks about all of it.
export function configureSandbox(addr: Address, limits: SandboxLimits): Command {
  return { configure_building: { ...absent(addr), sandbox: limits, idem: mintIdem() } };
}

// The unified building catalog, written as one user save. Empty reference,
// pin and Reading Room tables are explicit empty selections, not inheritance.
export function configureCatalog(addr: Address, selection: CatalogSelection): Command {
  return {
    configure_building: {
      ...absent(addr),
      catalog: selection.catalog,
      mcp_references: [...selection.mcp_references],
      skill_pins: { ...selection.skill_pins },
      reading_room: [...selection.reading_room],
      idem: mintIdem(),
    },
  };
}
