// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { QUERIES } from "../../core/asking";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, RunId } from "../../wire";
import { barOf, contextOf, lockOfConfig, remindersOf } from "../talk/gauge";

export default function ContextBar(props: { readonly room: Address; readonly run: RunId }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const config = storeValue(() => u.conn.asking.ask({ config: { addr: props.room } }));
  const rounds = storeValue(() => u.conn.asking.ask({ rounds: { run: props.run } }));
  const bar = createMemo(() => {
    const report = rounds();
    const served = endpoints();
    return barOf(
      lang(),
      contextOf(
        report !== undefined && "rounds" in report ? report.rounds.turns : [],
        served !== undefined && "endpoints" in served ? served.endpoints.endpoints : [],
        remindersOf(config()),
        lockOfConfig(config()),
      ),
    );
  });
  return (
    <Show when={bar()}>
      {(part) => (
        <span class="col-start-2 col-end-4 mt-snug">
          <span class="relative block h-hair rounded-pill bg-edge-panel" aria-hidden="true">
            <span
              class="refrain-session-context-used absolute inset-y-0 left-0 rounded-pill bg-text-quiet"
              style={{ width: `${part().share}%` }}
            />
            <Show when={part().handoff}>
              {(mark) => (
                <i class="refrain-session-context-mark" style={{ left: `${mark().at}%` }} />
              )}
            </Show>
          </span>
          <span class="sr-only">{part().said}</span>
        </span>
      )}
    </Show>
  );
}
