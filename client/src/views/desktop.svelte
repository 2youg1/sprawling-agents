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
  import { readDocument } from "../core/document";
  import { fill, say } from "../core/lang";
  import { ui } from "../ui";
  import type { Query } from "../wire";
  import Button from "./parts/button.svelte";

  interface Props {
    readonly addr: Address;
  }

  const { addr }: Props = $props();

  const u = ui();
  const lang = u.lang;

  let draft = $state("");
  let edited = $state(false);

  const question = $derived<Query>({ document: { at: desktopScopeAt(addr) } });
  const held = $derived(u.conn.asking.ask(question));
  // The city answers `unavailable` both for a building with no allowlist
  // yet and for a file it could not read, so the box starts empty for
  // either and the page says what is missing and how a save creates it,
  // rather than calling the allowlist empty or printing the question the
  // city could not answer: a save writes the file whole either way, and
  // the server reads an unreadable file as a closed door.
  const read = $derived(readDocument($held));
  const onDisk = $derived(read.kind === "held" ? read.value.text : "");

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
  {#if read.kind === "unavailable"}
    <p class="text-note text-text-faint">{fill(say($lang, "desktop_missing"), { path: desktopScopeAt(addr) })}</p>
  {/if}
  <div class="flex items-center gap-base">
    <Button
      label={say($lang, "desktop_save")}
      tone="primary"
      {...(edited ? {} : { why: say($lang, "desktop_unchanged") })}
      onPress={save}
    />
    {#if !edited && read.kind === "held" && onDisk === ""}
      <span class="text-note text-text-faint">{say($lang, "desktop_none")}</span>
    {/if}
    <span class="flex-1"></span>
    <code class="truncate font-mono text-note text-text-faint">{desktopScopeAt(addr)}</code>
  </div>
</div>
