// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";

import { ADMISSIONS, LANDINGS, WRITE_LIMITS } from "../../core/commands";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Card from "../setup/card_view";

interface Row {
  readonly term: Key;
  readonly says: Key;
  readonly values: readonly string[];
}

const ROWS: readonly Row[] = [
  { term: "admission_write", says: "admission_write_says", values: WRITE_LIMITS },
  { term: "admission_require", says: "admission_require_says", values: ADMISSIONS },
  { term: "admission_landing", says: "admission_landing_says", values: LANDINGS },
];

export default function Admission() {
  const lang = storeValue(ui().lang);
  const given = (key: Key) => say(lang(), key);
  return (
    <Card title={given("admission_title")} note={given("admission_note")}>
      <dl class="flex flex-col gap-snug">
        <For each={ROWS}>
          {(row) => (
            <div class="flex flex-col gap-hair border-t border-edge pt-snug">
              <dt class="text-note font-label text-text">{given(row.term)}</dt>
              <dd class="text-note text-text-quiet">{given(row.says)}</dd>
              <dd class="flex flex-wrap gap-snug font-mono text-note text-text-faint">
                <For each={row.values}>{(value) => <code>{value}</code>}</For>
              </dd>
            </div>
          )}
        </For>
      </dl>
      <p class="text-note text-text-faint">{given("admission_apart")}</p>
    </Card>
  );
}
