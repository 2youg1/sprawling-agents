// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { clock } from "../../core/time";
import { ui } from "../../ui";
import type { RegistryLine } from "../../wire";
import AskMayor from "../city/ask_mayor.solid";
import EmptyState from "../parts/empty.solid";
import Table, { type Column } from "../parts/table.solid";

export interface RegistryTableProps {
  readonly assets: readonly RegistryLine[];
}

function keyOf(line: RegistryLine): string {
  return `${line.at}\u0000${line.addr}\u0000${line.kind}\u0000${line.subject}`;
}

export default function RegistryTable(props: RegistryTableProps) {
  const lang = storeValue(ui().lang);
  const lines = createMemo(() => [...props.assets].sort((a, b) => b.at - a.at));
  const columns = createMemo<readonly Column<RegistryLine>[]>(() => [
    {
      key: "at",
      header: say(lang(), "registry_when"),
      summary: true,
      min: "12ch",
      render: (row) => (
        <span class="whitespace-nowrap text-note text-text-faint">{clock(lang(), row.at)}</span>
      ),
      compare: (a, b) => a.at - b.at,
    },
    {
      key: "kind",
      header: say(lang(), "registry_kind"),
      summary: true,
      min: "12ch",
      render: (row) => (
        <span class="whitespace-nowrap font-mono text-note text-text-quiet">{row.kind}</span>
      ),
      compare: (a, b) => a.kind.localeCompare(b.kind),
    },
    {
      key: "addr",
      header: say(lang(), "registry_where"),
      min: "24ch",
      render: (row) => (
        <a
          class="block truncate font-mono text-note text-text-quiet underline decoration-edge underline-offset-2 transition-colors ease-leave hover:decoration-accent hover:ease-arrive"
          href={toFragment({ kind: "building", address: row.addr })}
        >
          {row.addr}
        </a>
      ),
      compare: (a, b) => a.addr.localeCompare(b.addr),
    },
    {
      key: "subject",
      header: say(lang(), "registry_what"),
      min: "12ch",
      render: (row) => <span class="line-clamp-2 min-w-0 text-note text-text">{row.subject}</span>,
      compare: (a, b) => a.subject.localeCompare(b.subject),
    },
  ]);
  return (
    <Table
      caption={say(lang(), "nav_registry")}
      rows={lines()}
      keyOf={keyOf}
      columns={columns()}
      empty={() => <EmptyState missing="registry_empty" seat="inset" action={<AskMayor />} />}
    />
  );
}
