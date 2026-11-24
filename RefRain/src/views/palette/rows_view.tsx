// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { keymap, marks, type Action } from "../../core/keys";
import { storeValue } from "../../core/solid_store";
import { KbdMarks } from "../parts/kbd.look";
import type { RowLook, RowsLook } from "./rows";

export function PaletteRows(props: { readonly look: RowsLook }) {
  const keys = keymap();
  const bound = storeValue(keys.bound);
  const chord = (action: Action) => marks(bound()[action], keys.platform);
  return (
    <div {...props.look.list} class="max-h-palette overflow-y-auto">
      <For each={props.look.sections} keyed={(section) => section.key}>
        {(section) => (
          <div {...section().wire}>
            <Show when={section().heading}>
              {(heading) => (
                <div
                  {...heading().wire}
                  class="px-base pt-snug text-label font-label text-text-faint"
                >
                  {heading().text}
                </div>
              )}
            </Show>
            <For each={section().rows} keyed={(row) => row.key}>
              {(row) => <PaletteRow row={row()} marks={chord} />}
            </For>
          </div>
        )}
      </For>
    </div>
  );
}

function PaletteRow(props: {
  readonly row: RowLook;
  readonly marks: (action: Action) => readonly string[];
}) {
  const ending = createMemo(() => props.row.end);
  const shortcut = createMemo(() => {
    const end = ending();
    return end.kind === "chord" ? props.marks(end.action) : undefined;
  });
  const hint = () => {
    const end = ending();
    return end.kind === "hint" ? end.text : undefined;
  };
  return (
    <div
      id={props.row.wire.id}
      role={props.row.wire.role}
      aria-selected={props.row.wire["aria-selected"] ? "true" : "false"}
      aria-disabled={props.row.wire["aria-disabled"] ? "true" : "false"}
      onMouseEnter={() => props.row.wire.onmouseenter()}
      onClick={() => props.row.wire.onclick()}
      class={[
        "flex w-full cursor-pointer items-center justify-between gap-snug rounded-control px-base py-snug text-left text-body",
        "transition-colors ease-leave aria-disabled:text-text-disabled",
        props.row.refused ? "flex-wrap" : "hover:bg-raised-hover hover:ease-arrive",
        { "bg-raised-hover ease-arrive": props.row.active },
      ]}
    >
      <span class="max-w-full shrink-0 truncate font-mono">{props.row.label}</span>
      <span
        class={[
          "text-note text-text-faint",
          ending().kind === "chord"
            ? "shrink-0"
            : props.row.refused
              ? "min-w-0"
              : "min-w-0 shrink-1000 truncate",
        ]}
      >
        <Show when={shortcut()} fallback={hint()}>
          {(keys) => <KbdMarks marks={keys()} />}
        </Show>
      </span>
    </div>
  );
}
