// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Address, McpServer, McpServerHealth } from "../../wire";
import type { Intake } from "./draft";
import { configureMcp } from "../../core/commands";
import { ui } from "../../ui";
import { get, readonly, writable, type Readable } from "../../core/store";

interface McpState {
  readonly servers: readonly McpServer[];
  readonly health: readonly McpServerHealth[];
}

export interface Reach {
  readonly state: Readable<McpState>;
  readonly watch: (addr: Address) => () => void;
  readonly servers: () => readonly McpServer[];
  readonly health: () => readonly McpServerHealth[];
  readonly check: () => void;
  readonly intake: Intake;
  readonly withdraw: (label: string) => void;
}

export interface Scope {
  readonly addr: Address;
  readonly why: string | null;
}

export function reachOf(scope: Scope): Reach {
  const u = ui();
  const state = writable<McpState>({ servers: [], health: [] });

  function watch(addr: Address): () => void {
    state.set({ servers: [], health: [] });
    const stopBuilding = u.conn.asking.ask({ building_view: { addr } }).subscribe((answer) => {
      state.update((held) => ({
        ...held,
        servers: answer !== undefined && "building" in answer ? answer.building.mcp : [],
      }));
    });
    const stopHealth = u.conn.asking.ask({ mcp_health: { addr } }).subscribe((answer) => {
      state.update((held) => ({
        ...held,
        health: answer !== undefined && "mcp_health" in answer ? answer.mcp_health.servers : [],
      }));
    });
    return () => {
      stopBuilding();
      stopHealth();
    };
  }

  const check = (): void => {
    u.conn.asking.refresh({ mcp_health: { addr: scope.addr } });
  };

  const configure = (next: readonly McpServer[]): boolean => {
    const sent = u.send(configureMcp(scope.addr, next));

    if (sent) check();
    return sent;
  };
  const offerMany = (servers: readonly McpServer[]): boolean =>
    configure([...get(state).servers, ...servers]);
  return {
    state: readonly(state),
    watch,
    servers: () => get(state).servers,
    health: () => get(state).health,
    check,
    intake: {
      taken: () => get(state).servers.map((server) => server.label),
      offer: (server) => offerMany([server]),
      offerMany,
      why: () => scope.why,
    },
    withdraw: (label) => {
      configure(get(state).servers.filter((each) => each.label !== label));
    },
  };
}
