// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Address, McpServer, McpServerHealth } from "../../wire";
import type { Intake } from "./draft";
import { configureMcp } from "../../core/commands";
import { ui } from "../../ui";

export interface Reach {
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
  let servers = $state<readonly McpServer[]>([]);
  let health = $state<readonly McpServerHealth[]>([]);

  $effect(() => {
    const held = u.conn.asking.ask({ building_view: { addr: scope.addr } });
    return held.subscribe((answer) => {
      servers = answer !== undefined && "building" in answer ? answer.building.mcp : [];
    });
  });

  $effect(() => {
    const asked = u.conn.asking.ask({ mcp_health: { addr: scope.addr } });
    return asked.subscribe((answer) => {
      health = answer !== undefined && "mcp_health" in answer ? answer.mcp_health.servers : [];
    });
  });

  const check = (): void => {
    u.conn.asking.refresh({ mcp_health: { addr: scope.addr } });
  };

  const configure = (next: readonly McpServer[]): boolean => {
    const sent = u.send(configureMcp(scope.addr, next));

    if (sent) check();
    return sent;
  };
  return {
    servers: () => servers,
    health: () => health,
    check,
    intake: {
      taken: () => servers.map((server) => server.label),
      offer: (server) => configure([...servers, server]),
      why: () => scope.why,
    },
    withdraw: (label) => {
      configure(servers.filter((each) => each.label !== label));
    },
  };
}
