// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ago } from "../../core/time";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import { kindSaid, waitingSaid, type InboxLook, type InboxRow } from "./inbox";
import { called, residentAt } from "./naming";

export function InboxLookView(props: InboxLook) {
  return (
    <section class="text-note" aria-label={props.label}>
      <h2 class="text-label font-label text-text-quiet">{props.heading}</h2>
      <ul>
        <For each={props.rows} keyed={(row) => row.key}>
          {(row) => (
            <li>
              <a
                href={row().href}
                class="flex items-baseline gap-base rounded-control py-tight hover:bg-raised-hover"
              >
                <span class="shrink-0 text-text-quiet">{row().kind}</span>
                <span class="min-w-0 flex-1 truncate">{row().first}</span>
                <span class="max-w-3/10 truncate text-text-faint">{row().from}</span>
                <span class="shrink-0 text-text-faint">{row().ago}</span>
              </a>
            </li>
          )}
        </For>
      </ul>
    </section>
  );
}

export default function Inbox(props: { readonly addr: Address }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const answer = storeValue(() => u.conn.asking.ask({ inbox_view: { addr: props.addr } }));
  const rows = createMemo((): InboxRow[] => {
    const value = answer();
    if (value === undefined || !("inbox" in value)) return [];
    return value.inbox.waiting.map((line) => {
      const sender = residentAt(line.from);
      return {
        key: line.id,
        href: sender === null ? undefined : toFragment({ kind: "talk", address: sender }),
        kind: kindSaid(lang(), line.kind),
        first: line.first_line ?? "",
        from: sender === null ? line.from : called(sender, null, lang()),
        ago: ago(lang(), line.at, u.now()),
      };
    });
  });
  return (
    <Show when={rows().length > 0}>
      <div class="my-pane">
        <InboxLookView
          label={say(lang(), "inbox_waiting")}
          heading={waitingSaid(lang(), rows().length)}
          rows={rows()}
        />
      </div>
    </Show>
  );
}
