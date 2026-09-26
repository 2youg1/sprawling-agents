<script lang="ts" module>
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

import { Address } from "../wire";

// Where the file lives, as the city spells it. One authority on this
// side too: a page that joined its own path could join one that leaves
// the subtree.
export function desktopScopeAt(addr: Address): Address {
  return Address.make(`${addr}/.sprawling/DESKTOP.toml`);
}
</script>

<script lang="ts">
  import { untrack } from "svelte";

  import { configureDesktop } from "../core/commands";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import Button from "./parts/button.svelte";

  interface Props {
    readonly addr: Address;
  }

  const { addr }: Props = $props();

  const u = ui();
  const lang = u.lang;

  let draft = $state("");
  let edited = $state(false);

  const held = $derived(u.conn.asking.ask({ document: { at: desktopScopeAt(addr) } }));
  // What the city holds, or the empty string for a building that has no
  // allowlist yet. A building with none permits nothing, which is the
  // same thing an empty file says.
  const onDisk = $derived.by((): string => {
    const answer = $held;
    return answer !== undefined && "document" in answer ? answer.document.text : "";
  });

  // The box follows the file until somebody types in it, and follows it
  // again once their text has landed. A draft that outlived its save
  // would show a person their own words beside a file that no longer
  // says them. Whether somebody has typed is read without tracking: a
  // save clearing that flag must not pull the old file back into the
  // box while the fresh answer is still on its way.
  let was: Address | undefined = undefined;
  $effect(() => {
    const at = addr;
    const text = onDisk;
    if (!untrack(() => edited) || was !== at) {
      edited = false;
      draft = text;
    }
    was = at;
  });

  function save(): void {
    if (u.send(configureDesktop(addr, draft))) {
      edited = false;
    }
  }
</script>

<div class="flex flex-col gap-snug">
  <textarea
    class="min-h-output w-full rounded-control bg-raised px-base py-snug font-mono text-note text-text placeholder:text-text-faint"
    aria-label={say($lang, "desktop_allowlist")}
    placeholder={say($lang, "desktop_empty")}
    bind:value={draft}
    oninput={() => {
      edited = true;
    }}
  ></textarea>
  <div class="flex items-center gap-base">
    <Button
      label={say($lang, "desktop_save")}
      tone="primary"
      {...(edited ? {} : { why: say($lang, "desktop_unchanged") })}
      onPress={save}
    />
    {#if !edited && onDisk === ""}
      <span class="text-note text-text-faint">{say($lang, "desktop_none")}</span>
    {/if}
    <span class="flex-1"></span>
    <code class="truncate font-mono text-note text-text-faint">{desktopScopeAt(addr)}</code>
  </div>
</div>
