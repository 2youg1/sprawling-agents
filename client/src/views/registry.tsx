// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this city has decided is worth keeping: one line per asset a run
// filed, with when it was filed, what kind of thing it is, the building
// it belongs to, and what it is about.
//
// The registry is the one place an external event that outlived its run
// can be found, and until this screen existed `Query::RegistryView` had
// no reader at all - a question the city answers and nobody asks is a
// defect of the same family as an answer nobody draws.
//
// Newest first, because the question a person opens a registry with is
// what was filed last; every column that can be ordered by reorders it
// from there, which is `parts/table.tsx`'s own behaviour and not a
// second sort written here.

import { Show, createMemo } from "solid-js";

import { QUERIES } from "../core/asking";
import { MAYOR, toFragment } from "../core/route";
import { clock } from "../core/time";
import type { RegistryLine } from "../wire";
import { useLang, useSay, useUi } from "../ui";
import { EmptyState } from "./parts/empty";
import { Table, type Column } from "./parts/table";

// The identity of one line. The wire gives an asset no id of its own,
// and these four fields are what the city wrote about it, so the four
// together are as much identity as the record carries.
function keyOf(line: RegistryLine): string {
  return `${String(line.at)}\u0000${line.addr}\u0000${line.kind}\u0000${line.subject}`;
}

// One answer, drawn. Separate from the ask so the gallery can show a
// registry in states nobody's own city happens to be in - full and
// empty - which is the same split `views/machine.tsx` makes.
export function RegistryTable(props: { readonly assets: readonly RegistryLine[] }) {
  const say = useSay();
  const lang = useLang();
  const lines = createMemo<readonly RegistryLine[]>(() =>
    [...props.assets].sort((a, b) => b.at - a.at),
  );
  const columns = createMemo<readonly Column<RegistryLine>[]>(() => [
    {
      key: "at",
      header: say("registry_when"),
      summary: true,
      render: (line) => <span class="whitespace-nowrap text-note text-text-faint">{clock(lang(), line.at)}</span>,
      compare: (a, b) => a.at - b.at,
    },
    {
      key: "kind",
      header: say("registry_kind"),
      summary: true,
      // The city's own word for what was filed, which no language
      // translates because it is the event's own spelling.
      render: (line) => <span class="whitespace-nowrap font-mono text-note text-text-quiet">{line.kind}</span>,
      compare: (a, b) => a.kind.localeCompare(b.kind),
    },
    {
      key: "addr",
      header: say("registry_where"),
      render: (line) => (
        <a
          href={toFragment({ kind: "building", address: line.addr })}
          class="font-mono text-note text-text-quiet underline decoration-g3 underline-offset-2 hover:decoration-accent"
        >
          {line.addr}
        </a>
      ),
      compare: (a, b) => a.addr.localeCompare(b.addr),
    },
    {
      key: "subject",
      header: say("registry_what"),
      render: (line) => <span class="block min-w-0 wrap-anywhere text-note text-text">{line.subject}</span>,
      compare: (a, b) => a.subject.localeCompare(b.subject),
    },
  ]);

  return (
    <Table
      caption={say("nav_registry")}
      columns={columns()}
      rows={lines()}
      keyOf={keyOf}
      empty={
        // An empty registry is a city that has not been asked for
        // anything yet: an asset is filed by a run, and a run begins
        // in the conversation with the Mayor.
        <EmptyState
          text={say("registry_empty")}
          action={
            <a
              href={toFragment({ kind: "talk", address: MAYOR })}
              class="rounded-control bg-accent px-base py-snug text-label text-g0 hover:bg-accent-hover"
            >
              {say("city_ask_mayor")}
            </a>
          }
        />
      }
    />
  );
}

// The screen: the one question, and the table that draws its answer.
export function Registry() {
  const ui = useUi();
  const say = useSay();
  const asked = ui.conn.asking.ask(QUERIES.registry);
  const answer = createMemo(() => {
    const held = asked();
    return held !== undefined && "registry" in held ? held.registry : undefined;
  });
  return (
    <div class="mx-auto w-full max-w-page px-pane py-wide">
      <h1 class="mb-wide text-title font-title">{say("nav_registry")}</h1>
      <Show when={answer()} fallback={<p class="text-text-disabled">…</p>}>
        {(held) => <RegistryTable assets={held().assets} />}
      </Show>
    </div>
  );
}
