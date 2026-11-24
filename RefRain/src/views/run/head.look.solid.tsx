// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Switch, createMemo } from "solid-js";
import type { Face, HeadLook, LineLook } from "./head";

const FACE: Record<Face, string> = { words: "", figure: "figure", mono: "font-mono" };

type LineKind = LineLook["kind"];

function variant<K extends LineKind>(
  line: LineLook,
  kind: K,
): line is Extract<LineLook, { kind: K }> {
  return line.kind === kind;
}

function Line(props: { readonly line: LineLook }) {
  function read<K extends LineKind>(kind: K) {
    const value = props.line;
    return variant(value, kind) ? value : undefined;
  }
  const text = createMemo(() => read("text"));
  const time = createMemo(() => read("time"));
  const place = createMemo(() => read("place"));
  return (
    <Switch>
      <Match when={text()}>
        {(item) => (
          <dd
            class={[
              "truncate",
              FACE[item().face],
              item().rank === "value" ? "text-text" : "text-note text-text-quiet",
            ]}
          >
            {item().text}
          </dd>
        )}
      </Match>
      <Match when={time()}>
        {(item) => (
          <dd class="figure truncate text-note text-text-quiet">
            <time datetime={item().at}>{item().text}</time>
          </dd>
        )}
      </Match>
      <Match when={place()}>
        {(item) => (
          <dd class="truncate font-mono">
            <a
              {...item().building.wire}
              class="text-text-quiet transition-colors ease-leave still:transition-none hover:text-text hover:ease-arrive"
            >
              {item().building.text}
            </a>
            <span class="text-text-faint">{"/"}</span>
            <a {...item().room.wire} class="text-text">
              {item().room.text}
            </a>
          </dd>
        )}
      </Match>
    </Switch>
  );
}

export default function HeadLookView(look: HeadLook) {
  return (
    <dl
      class="grid grid-cols-[repeat(auto-fill,minmax(22ch,1fr))] gap-x-gutter gap-y-base border-y border-edge py-base"
      aria-label={look.label}
    >
      <For each={look.cells} keyed={(cell) => cell.key}>
        {(cell) => (
          <div class={["flex min-w-0 flex-col", cell().wide && "col-span-2"]}>
            <dt class="text-note text-text-faint">{cell().name}</dt>
            <For each={cell().lines} keyed={(line) => line.key}>
              {(line) => <Line line={line()} />}
            </For>
          </div>
        )}
      </For>
    </dl>
  );
}
