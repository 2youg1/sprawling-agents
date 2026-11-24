// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { fill, say } from "../../core/lang";
import { producedOf } from "../../core/results";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { GitOid } from "../../wire";

export interface ProducedProps {
  readonly base: GitOid;
  readonly head: GitOid | null;
}

export default function Produced(props: ProducedProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(() =>
    u.conn.asking.ask({ changes: { base: props.base, head: props.head } }),
  );
  const produced = createMemo(() => {
    const held = asked();
    if (held === undefined || !("changes" in held)) return null;
    const tally = producedOf(held.changes.files);
    return tally.files > 0 ? tally : null;
  });
  return (
    <Show when={produced()}>
      {(tally) => (
        <span
          class="shrink-0 text-note text-text-quiet"
          aria-label={say(lang(), "results_produced")}
        >
          {fill(say(lang(), "results_row_produced"), {
            files: String(tally().files),
            added: String(tally().added),
            removed: String(tally().removed),
          })}
        </span>
      )}
    </Show>
  );
}
