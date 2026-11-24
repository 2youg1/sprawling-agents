// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { lookOf } from "./drop_refused";
import type { Refused } from "./dropping";

export default function DropRefused(props: { readonly refused: readonly Refused[] }) {
  const lang = storeValue(ui().lang);
  const look = createMemo(() => lookOf(props.refused, lang()));
  const lines = createMemo(() => look().lines.map((text, index) => ({ text, index })));
  return (
    <For each={lines()} keyed={(line) => line.index}>
      {(line) => (
        <p class="text-note text-alert" role={look().wire.role}>
          {line().text}
        </p>
      )}
    </For>
  );
}
