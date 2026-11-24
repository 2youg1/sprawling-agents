// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch, createMemo } from "solid-js";
import { readAnswer } from "../../core/answered";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { EvidenceKind, Query, RunId } from "../../wire";
import EmptyState from "../parts/empty.solid";
import Unanswered from "../parts/unanswered.solid";

const WORD: Record<EvidenceKind, Key> = {
  screenshot: "evidence_screenshot",
  finished: "evidence_finished",
};

export default function Evidence(props: { readonly run: RunId }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const query = createMemo<Query>(() => ({ evidence: { run: props.run } }));
  const answer = storeValue(() => u.conn.asking.ask(query()));
  const read = createMemo(() =>
    readAnswer(answer(), (held) => ("evidence" in held ? held.evidence.items : undefined)),
  );
  const blocked = () => {
    const held = read();
    return held.kind === "unavailable" ? held.query : undefined;
  };
  const items = () => {
    const held = read();
    return held.kind === "held" ? held.value : undefined;
  };
  return (
    <Switch fallback={<p class="text-text-faint">…</p>}>
      <Match when={blocked()}>{(kind) => <Unanswered query={kind()} asked={query()} />}</Match>
      <Match when={items()?.length === 0}>
        <EmptyState missing="run_no_evidence" />
      </Match>
      <Match when={items()}>
        {(rows) => (
          <ul class="text-note">
            <For each={rows()} keyed={(item) => item.at}>
              {(item) => (
                <li class="flex items-center gap-base border-b border-edge py-snug">
                  <span class="w-figure shrink-0 text-text-faint">
                    {say(lang(), WORD[item().kind])}
                  </span>
                  <span class="flex-1 truncate font-mono text-text-quiet">{item().locator}</span>
                  <Show when={item().picture}>
                    {(picture) => (
                      <span class="figure text-text-faint">
                        {picture().width}×{picture().height} {picture().media_type}
                      </span>
                    )}
                  </Show>
                  <span class="figure text-text-faint">#{item().at}</span>
                </li>
              )}
            </For>
          </ul>
        )}
      </Match>
    </Switch>
  );
}
