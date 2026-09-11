// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The windows on this person's own machine a building's connector may
// touch, one building at a time.
//
// One box holding the whole file, because that is what the file is: the
// connector reads it whole at start-up and permits nothing it cannot
// read, so a form with a field per window would be a second reading of
// a syntax this side does not own (city-SPEC.md 8-26).

import { Show, createEffect, createMemo, createSignal } from "solid-js";

import { Address } from "../core/address";
import { configureDesktop } from "../core/commands";
import { useCommand, useSay, useUi } from "../ui";

// Where the file lives, as the city spells it. One authority on this
// side too: a page that joined its own path could join one that leaves
// the subtree.
export function desktopScopeAt(addr: Address): Address {
  return Address(`${addr}/.sprawling/DESKTOP.toml`);
}

export function DesktopForm(props: { readonly addr: Address }) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const [draft, setDraft] = createSignal("");
  const [edited, setEdited] = createSignal(false);

  const held = createMemo(() =>
    ui.conn.asking.ask({ document: { at: desktopScopeAt(props.addr) } }),
  );
  // What the city holds, or the empty string for a building that has no
  // allowlist yet. A building with none permits nothing, which is the
  // same thing an empty file says.
  const onDisk = createMemo(() => {
    const answer = held()();
    return answer !== undefined && "document" in answer ? answer.document.text : "";
  });
  // The box follows the file until somebody types in it, and follows it
  // again once their text has landed. A draft that outlived its save
  // would show a person their own words beside a file that no longer
  // says them.
  createEffect((was: Address | undefined) => {
    const at = props.addr;
    const text = onDisk();
    if (!edited() || was !== at) {
      setEdited(false);
      setDraft(text);
    }
    return at;
  }, undefined);

  const save = () => {
    if (command(configureDesktop(props.addr, draft()))) {
      setEdited(false);
    }
  };

  return (
    <div class="flex flex-col gap-snug">
      <textarea
        class="min-h-output w-full rounded-control bg-g2 px-base py-snug font-mono text-note text-text outline-none placeholder:text-text-disabled"
        aria-label={say("desktop_allowlist")}
        placeholder={say("desktop_empty")}
        value={draft()}
        onInput={(event) => {
          setEdited(true);
          setDraft(event.currentTarget.value);
        }}
      />
      <div class="flex items-center gap-base">
        <button
          type="button"
          class="rounded-control bg-accent px-base py-tight text-label text-g0 hover:bg-accent-hover disabled:bg-g3 disabled:text-text-disabled"
          disabled={!edited()}
          onClick={save}
        >
          {say("desktop_save")}
        </button>
        <Show when={!edited() && onDisk() === ""}>
          <span class="text-note text-text-disabled">{say("desktop_none")}</span>
        </Show>
        <span class="flex-1" />
        <code class="truncate font-mono text-note text-text-disabled">
          {desktopScopeAt(props.addr)}
        </code>
      </div>
    </div>
  );
}
