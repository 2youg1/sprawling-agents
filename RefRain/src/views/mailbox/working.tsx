// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import type { RunBelief } from "../../core/belief";
import { say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import type { Weight } from "../parts/glyph";
import Glyph from "../parts/glyph.solid";
import { ticker } from "../talk/timing";
import Section from "./section";
import { rowOf, type WorkingRowLook } from "./working_row";

const INK: Readonly<Record<Weight, string>> = {
  quiet: "text-text-quiet",
  live: "text-accent",
  alert: "text-alert",
};

export function WorkingRow(props: WorkingRowLook) {
  return (
    <li>
      <a
        class="grid grid-cols-[var(--spacing-glyph-sm)_minmax(0,1fr)_auto_auto] items-center gap-x-snug rounded-card px-snug py-tight hover:wash focus-visible:wash"
        href={props.link.href ?? undefined}
        data-entry=""
        onClick={(event) => {
          props.link.onclick?.call(event.currentTarget, event);
        }}
      >
        <Glyph name={props.mark.glyph} size="sm" class={INK[props.mark.weight]} />
        <span class="min-w-0 truncate font-label">{props.room}</span>
        <span class="figure text-note text-text-faint">{props.doing}</span>
        <kbd class="entry-n" aria-hidden="true" />
        <Show when={props.task}>
          {(task) => (
            <span class="col-start-2 col-end-5 truncate text-note text-text-quiet">{task()}</span>
          )}
        </Show>
      </a>
    </li>
  );
}

function link(room: Address, run: RunBelief): string {
  return toFragment(
    run.sessionBegan === undefined
      ? { kind: "talk", address: room }
      : { kind: "talk", address: room, session: run.sessionBegan },
  );
}

export default function Working(props: { readonly onLeave: () => void }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const now = storeValue(ticker(u.now));
  const rows = createMemo(() => {
    const newest = new Map<Address, RunBelief>();
    for (const run of belief().live) if (run.addr !== null) newest.set(run.addr, run);
    return [...newest.entries()].sort((a, b) => (b[1].started ?? 0) - (a[1].started ?? 0));
  });
  return (
    <Section
      title={say(lang(), "mailbox_working")}
      empty={say(lang(), "mailbox_working_none")}
      count={rows().length}
    >
      <ul>
        <For each={rows()} keyed={([, run]) => run.run}>
          {(entry) => (
            <WorkingRow
              {...rowOf(entry()[1], entry()[0], lang(), now(), {
                href: link(entry()[0], entry()[1]),
                onclick: props.onLeave,
              })}
            />
          )}
        </For>
      </ul>
    </Section>
  );
}
