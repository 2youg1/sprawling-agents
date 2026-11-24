// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For } from "solid-js";

import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import type { Draft } from "./draft";
import { readingsOf } from "./preview";

interface PreviewProps {
  readonly draft: Draft;
  readonly reference: string;
  readonly model: string;
}

export default function Previews(props: PreviewProps) {
  const lang = storeValue(ui().lang);
  const readings = createMemo(() => readingsOf(props.draft, props.reference, props.model, lang()));
  return (
    <For each={readings()} keyed={(entry) => entry.key}>
      {(entry) => (
        <details class="rounded-card bg-chrome px-base py-snug text-note">
          <summary
            class="w-fit cursor-pointer rounded-control px-tight text-text-quiet"
            data-control="surface"
          >
            {entry().title}
          </summary>
          <pre
            class={[
              "mt-snug overflow-auto font-mono text-text-faint",
              entry().wraps && "whitespace-pre-wrap",
            ]}
          >
            {entry().text}
          </pre>
        </details>
      )}
    </For>
  );
}
