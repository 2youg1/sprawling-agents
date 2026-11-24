// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Show } from "solid-js";
import { drawMath, formulaOf } from "./formats/math";

export interface FormulaProps {
  readonly source: string;
}

export default function Formula(props: FormulaProps) {
  const formula = createMemo(() => formulaOf(props.source));
  const [host, setHost] = createSignal<HTMLSpanElement>();
  const [drawn, setDrawn] = createSignal(false);
  createEffect(
    () => [host(), formula()] as const,
    ([node, shown]) => {
      setDrawn(false);
      if (node === undefined || shown === null) return;
      let current = true;
      void drawMath(shown.tex, node, shown.placed, () => current).then(
        (ready) => {
          if (current) setDrawn(ready);
        },
        () => {
          if (current) setDrawn(false);
        },
      );
      return () => {
        current = false;
      };
    },
  );
  return (
    <>
      <span
        ref={(node) => setHost(node)}
        class={
          drawn() ? (formula()?.placed === "display" ? "block overflow-x-auto" : "") : "hidden"
        }
      />
      <Show when={!drawn()}>
        <code class="font-mono text-note text-text-faint">{props.source}</code>
      </Show>
    </>
  );
}
