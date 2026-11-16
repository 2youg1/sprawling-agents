// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

export function configureDesktop(addr: Address, allowlist: string): Command {
  return { configure_building: { ...absent(addr), desktop: allowlist, idem: mintIdem() } };
}

export function configureContext(addr: Address, percent: number): Command {
  return {
    configure_building: { ...absent(addr), context_second_threshold: percent, idem: mintIdem() },
  };
}

export function configureSandbox(addr: Address, limits: SandboxLimits): Command {
  return { configure_building: { ...absent(addr), sandbox: limits, idem: mintIdem() } };
}

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
