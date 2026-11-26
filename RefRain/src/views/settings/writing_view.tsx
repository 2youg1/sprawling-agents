// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";

import { say, type Key, type Lang } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import {
  INDENTATIONS,
  MEASURES,
  writingPreferences,
  type WritingIndentation,
  type WritingMeasure,
} from "../../core/writing";

const MEASURE: Record<WritingMeasure, Key> = {
  gold: "writing_gold",
  silver: "writing_silver",
  wide: "writing_wide",
};

const INDENTATION: Record<WritingIndentation, Key> = {
  off: "writing_indent_off",
  display: "writing_indent_display",
  source: "writing_indent_source",
};

export default function WritingView(props: { readonly lang: Lang }) {
  const prefs = writingPreferences();
  const held = storeValue(prefs.held);
  const word = (key: Key) => say(props.lang, key);
  return (
    <div class="flex flex-col gap-base rounded-card bg-raised px-base py-snug">
      <label class="flex flex-wrap items-center justify-between gap-snug">
        <span>{word("writing_measure")}</span>
        <select
          data-control=""
          class="rounded-control border border-edge bg-page px-snug py-tight"
          value={held().measure}
          onChange={(event) => {
            const value = MEASURES.find((each) => each === event.currentTarget.value);
            if (value !== undefined) prefs.measure(value);
          }}
        >
          <For each={MEASURES}>
            {(each) => <option value={each}>{word(MEASURE[each])}</option>}
          </For>
        </select>
      </label>
      <label class="flex flex-wrap items-center justify-between gap-snug">
        <span>{word("writing_indent")}</span>
        <select
          data-control=""
          class="rounded-control border border-edge bg-page px-snug py-tight"
          value={held().indentation}
          onChange={(event) => {
            const value = INDENTATIONS.find((each) => each === event.currentTarget.value);
            if (value !== undefined) prefs.indentation(value);
          }}
        >
          <For each={INDENTATIONS}>
            {(each) => <option value={each}>{word(INDENTATION[each])}</option>}
          </For>
        </select>
      </label>
      <label class="flex items-center gap-snug">
        <input
          type="checkbox"
          data-control=""
          checked={held().typewriter}
          onChange={(event) => prefs.typewriter(event.currentTarget.checked)}
        />
        {word("refrain_typewriter")}
      </label>
    </div>
  );
}
