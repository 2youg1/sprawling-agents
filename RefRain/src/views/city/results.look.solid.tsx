// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch } from "solid-js";
import Glyph from "../parts/glyph.solid";
import { OUTCOME_GLYPH, OUTCOME_INK } from "../shared/outcome";
import Produced from "./produced.solid";
import type { ResultRowLook, ResultsLook } from "./results";

function Row(props: { readonly row: ResultRowLook }) {
  const outcome = () => props.row.outcome;
  const tail = () => props.row.tail;
  const produced = () => {
    const last = tail();
    return last.kind === "produced" ? last : null;
  };
  const words = () => {
    const last = tail();
    return last.kind === "asks" || last.kind === "stopped" ? last : null;
  };
  return (
    <li
      class={[
        "border-b border-l-2 border-b-edge",
        outcome() === "waiting" ? "border-l-alert" : "border-l-transparent",
      ]}
    >
      <a
        href={props.row.wire.href}
        class="flex items-baseline gap-base px-tight py-snug text-note transition-colors ease-leave hover:wash hover:ease-arrive"
      >
        <span class="w-figure shrink-0 font-mono text-text-faint">{props.row.at}</span>
        <span class="inline-flex size-glyph-sm shrink-0 self-center">
          <Show when={outcome()}>
            {(kind) => <Glyph name={OUTCOME_GLYPH[kind()]} size="sm" class={OUTCOME_INK[kind()]} />}
          </Show>
        </span>
        <span class="w-output shrink-0 truncate font-mono text-text-quiet">{props.row.addr}</span>
        <span class="min-w-0 flex-1 truncate text-body text-text">{props.row.title}</span>
        <Switch>
          <Match when={produced()}>
            {(made) => (
              <>
                <Produced run={made().run} />
                <Show when={made().pr !== null}>
                  <span class="shrink-0 text-text-quiet">{made().pr}</span>
                </Show>
              </>
            )}
          </Match>
          <Match when={words()}>
            {(detail) => (
              <span
                class={
                  detail().kind === "asks"
                    ? "min-w-0 shrink truncate text-text"
                    : "shrink-0 text-text-quiet"
                }
              >
                {detail().text}
              </span>
            )}
          </Match>
        </Switch>
      </a>
    </li>
  );
}

export default function ResultsLookView(look: ResultsLook) {
  return (
    <div class="flex flex-col gap-wide">
      <Show when={look.none !== null}>
        <p class="text-note text-text-faint">{look.none}</p>
      </Show>
      <For each={look.bands} keyed={(band) => band.key}>
        {(band) => (
          <section aria-label={band().heading}>
            <h2 class="mb-tight border-b border-edge pb-tight text-note text-text-quiet">
              {band().heading}
              <span class="text-text-faint">{band().count}</span>
            </h2>
            <ul>
              <For each={band().rows} keyed={(row) => row.key}>
                {(row) => <Row row={row()} />}
              </For>
            </ul>
          </section>
        )}
      </For>
    </div>
  );
}
