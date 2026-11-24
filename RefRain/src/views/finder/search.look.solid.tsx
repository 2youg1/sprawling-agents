// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { SearchLook } from "./search";

export type PortProps = SearchLook;

export default function FinderSearchLook(look: SearchLook) {
  return (
    <>
      <h2 {...look.title.wire} class="px-snug text-label text-text-quiet">
        {look.title.text}
      </h2>
      <input
        role={look.box.role}
        aria-autocomplete={look.box["aria-autocomplete"]}
        aria-expanded={look.box["aria-expanded"] ? "true" : "false"}
        aria-controls={look.box["aria-controls"]}
        aria-activedescendant={look.box["aria-activedescendant"]}
        aria-labelledby={look.box["aria-labelledby"]}
        placeholder={look.box.placeholder}
        value={look.box.value}
        onInput={(event) => look.box.oninput(event)}
        onKeyDown={(event) => {
          look.box.onkeydown(event);
          if (event.defaultPrevented && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
            requestAnimationFrame(() => {
              const active = look.box["aria-activedescendant"];
              if (active !== undefined)
                document.getElementById(active)?.scrollIntoView({ block: "nearest" });
            });
          }
        }}
        class="w-full rounded-control bg-page px-base py-snug text-body placeholder:text-text-faint"
      />
      <ul {...look.list} class="max-h-palette overflow-y-auto">
        <For each={look.options} keyed={(entry) => entry.key}>
          {(entry) => (
            <li
              id={entry().wire.id}
              role={entry().wire.role}
              aria-selected={entry().wire["aria-selected"] ? "true" : "false"}
              onMouseEnter={() => entry().wire.onmouseenter()}
              onClick={() => entry().wire.onclick()}
              class={[
                "flex cursor-pointer items-baseline gap-snug rounded-control px-base py-snug font-mono text-body",
                "transition-colors ease-leave hover:bg-raised-hover hover:ease-arrive",
                entry().active ? "bg-raised-hover text-text ease-arrive" : "text-text-quiet",
              ]}
            >
              <span class="truncate">{entry().name}</span>
              <span class="min-w-0 truncate text-note text-text-faint">{entry().dir}</span>
            </li>
          )}
        </For>
      </ul>
      <For each={look.notes}>
        {(note) => <p class="px-snug text-note text-text-faint">{note}</p>}
      </For>
    </>
  );
}
