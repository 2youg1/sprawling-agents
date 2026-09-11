// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What waits for the person, as cards in the conversation rather than
// a page of their own: a question that stops a run belongs where the
// person is already looking. Identical questions are grouped by their
// cluster key so one answer covers one question; a tainted item is
// never grouped.

import { For, Show, createMemo } from "solid-js";

import { approve } from "../../core/commands";
import { ago } from "../../core/time";
import type { ApprovalItem } from "../../wire";
import { useCommand, useLang, useSay, useUi } from "../../ui";

interface Group {
  readonly key: string;
  readonly items: readonly ApprovalItem[];
}

function grouped(items: readonly ApprovalItem[]): Group[] {
  const groups = new Map<string, ApprovalItem[]>();
  for (const item of items) {
    const key = item.tainted
      ? `item:${item.id}`
      : `${item.cluster_key.class}:${item.cluster_key.detail}`;
    const held = groups.get(key) ?? [];
    held.push(item);
    groups.set(key, held);
  }
  return [...groups.entries()]
    .map(([key, held]) => ({ key, items: held.sort((a, b) => a.created - b.created) }))
    .sort((a, b) => (a.items[0]?.created ?? 0) - (b.items[0]?.created ?? 0));
}

export function Waiting() {
  const ui = useUi();
  const answer = ui.conn.asking.ask("approval_queue");
  const items = createMemo<readonly ApprovalItem[]>(() => {
    const held = answer();
    return held !== undefined && "approvals" in held ? held.approvals.items : [];
  });
  return <WaitingCards items={items()} />;
}

// The cards themselves. Separate from the ask so the gallery can show
// what a person waiting on a run sees without a run being blocked
// (docs/frontend-method.md).
export function WaitingCards(props: { readonly items: readonly ApprovalItem[] }) {
  const ui = useUi();
  const say = useSay();
  const lang = useLang();
  const command = useCommand();
  const groups = createMemo(() => grouped(props.items));

  return (
    <For each={groups()}>
      {(group) => {
        const first = () => group.items[0];
        return (
          <Show when={first()}>
            {(item) => (
              <div
                class="my-base rounded-panel border border-alert/50 bg-g1 px-pane py-base"
                role="group"
                aria-label={say("wait_title")}
              >
                <div class="flex items-center justify-between text-note">
                  <span class="text-alert">{say("wait_from", { actor: item().actor })}</span>
                  <span class="text-text-faint">{ago(lang(), item().created, ui.now())}</span>
                </div>
                <p class="my-snug text-body leading-relaxed">{item().action_desc}</p>
                <div class="flex flex-wrap items-center gap-snug text-note">
                  <Show when={group.items.length > 1}>
                    <span class="text-text-faint">{say("wait_same", { n: String(group.items.length) })}</span>
                  </Show>
                  <Show when={item().tainted}>
                    <span class="rounded-pill bg-g2 px-snug text-text-quiet">{say("wait_tainted")}</span>
                  </Show>
                  <span class="flex-1" />
                  <button
                    type="button"
                    class="rounded-control px-base py-tight text-label text-text-quiet hover:bg-g2"
                    onClick={() => {
                      for (const each of group.items) command(approve(each.id, "deny"));
                    }}
                  >
                    {say("wait_deny")}
                  </button>
                  <button
                    type="button"
                    class="rounded-control bg-accent px-base py-tight text-label text-g0 hover:bg-accent-hover"
                    onClick={() => {
                      for (const each of group.items) command(approve(each.id, "allow"));
                    }}
                  >
                    {say("wait_allow")}
                  </button>
                </div>
              </div>
            )}
          </Show>
        );
      }}
    </For>
  );
}
