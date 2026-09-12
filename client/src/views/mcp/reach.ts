// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one building reaches, and the one door that changes it.
//
// Adding and withdrawing are the same command - the city is told the
// whole list, never a difference - so both live here rather than being
// spelled again in every section that offers a button. Reconfiguring
// writes `building_configured`, and `staleBy` turns that record into an
// invalidation of this very answer: nothing below waits on a timer for
// the list to catch up.
//
// Where each server stands is asked for separately and never folded
// from a record, because it is a fact about now - a program that starts,
// a host that answers, an account that is still valid - and one
// handshake per server costs seconds.

import { createMemo } from "solid-js";

import type { Address, McpServer, McpServerHealth } from "../../wire";
import type { Intake } from "./draft";
import { configureMcp } from "../../core/commands";
import { useCommand, useUi } from "../../ui";

export interface Reach {
  readonly servers: () => readonly McpServer[];
  // Where each of them stands, once somebody has asked. Empty until the
  // first answer lands, and empty again for a scope nobody asked about.
  readonly health: () => readonly McpServerHealth[];
  // Reach every server again and say where each one stands. Explicit
  // rather than folded into a record's arrival, because one handshake
  // per server costs seconds and a person is waiting in front of it.
  readonly check: () => void;
  readonly intake: Intake;
  readonly withdraw: (label: string) => void;
}

/**
 * Which building, and what stops a send. Read as a props object rather
 * than as two accessors: a caller writes `get addr()`, the reads happen
 * where this module tracks them, and nothing reactive crosses the call
 * as a bare function.
 */
export interface Scope {
  readonly addr: Address;
  // Already in the person's language, or null when a send may go.
  readonly why: string | null;
}

export function reachOf(scope: Scope): Reach {
  const ui = useUi();
  const command = useCommand();
  const answer = createMemo(() => ui.conn.asking.ask({ building_view: { addr: scope.addr } }));
  const servers = createMemo<readonly McpServer[]>(() => {
    const held = answer()();
    return held !== undefined && "building" in held ? held.building.mcp : [];
  });
  const asked = createMemo(() => ui.conn.asking.ask({ mcp_health: { addr: scope.addr } }));
  const health = createMemo<readonly McpServerHealth[]>(() => {
    const held = asked()();
    return held !== undefined && "mcp_health" in held ? held.mcp_health.servers : [];
  });
  const check = () => {
    ui.conn.asking.refresh({ mcp_health: { addr: scope.addr } });
  };
  const configure = (next: readonly McpServer[]) => {
    const sent = command(configureMcp(scope.addr, next));
    // What a person wants to know next is whether the server they just
    // named answers, so the list is reached again rather than waiting
    // for somebody to press anything.
    if (sent) check();
    return sent;
  };
  return {
    servers,
    health,
    check,
    intake: {
      taken: () => servers().map((server) => server.label),
      offer: (server) => configure([...servers(), server]),
      why: () => scope.why,
    },
    withdraw: (label) => {
      configure(servers().filter((each) => each.label !== label));
    },
  };
}
