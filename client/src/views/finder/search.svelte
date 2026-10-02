<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What the file finder holds (client/Spec.lean §4-62): the building it looks
  // in, the box, the files the city found, and what the city could not
  // say. An APG Combobox: the box keeps the focus, ↓/↑ move the active
  // option, Enter opens it in the right side through `openDocument` at
  // the worktree's text. The page never claims that a directory it was
  // not told about holds no such name, so an answer the city cut short
  // says so under the list (`Query::Find`, `crates/wire/Spec.lean`
  // §8-82). The modal around it is `views/finder.svelte`'s, and opening
  // it puts the focus in the box, the first control the dialog holds.

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, FindAnswer } from "../../wire";
  import { openDocument } from "../inspect/open.svelte";

  interface Props {
    // The building looked in, named in the title.
    readonly under: Address;
    // The id of the title, which names the dialog around this too.
    readonly titleId: string;
    readonly onClose: () => void;
  }

  const { under, titleId, onClose }: Props = $props();

  const uid = $props.id();
  const u = ui();
  const { lang } = u;

  let text = $state("");
  let cursor = $state(0);

  const asked = $derived(u.conn.asking.ask({ find: { under, text: text.trim() } }));
  // The last answer stays drawn while the next one is asked, so the list
  // does not blink empty on every key.
  let found = $state.raw<FindAnswer | undefined>(undefined);
  $effect(() => {
    const answer = $asked;
    if (answer !== undefined && "find" in answer) found = answer.find;
  });
  const paths = $derived(found?.paths ?? []);

  function open(path: string | undefined): void {
    if (path === undefined) return;
    openDocument({ building: under, path, version: null });
    onClose();
  }

  function keys(event: KeyboardEvent): void {
    switch (event.key) {
      case "ArrowDown":
        cursor = Math.min(cursor + 1, paths.length - 1);
        break;
      case "ArrowUp":
        cursor = Math.max(cursor - 1, 0);
        break;
      case "Enter":
        open(paths[cursor]);
        break;
      default:
        return;
    }
    event.preventDefault();
  }

  // A path drawn as its name, then the directory it sits in.
  function parts(path: string): { readonly name: string; readonly dir: string } {
    const slash = path.lastIndexOf("/");
    return slash < 0 ? { name: path, dir: "" } : { name: path.slice(slash + 1), dir: path.slice(0, slash) };
  }
</script>

<h2 id={titleId} class="px-snug text-label text-text-quiet">{fill(say($lang, "finder_title"), { building: under })}</h2>
<input
  class="w-full rounded-control bg-page px-base py-snug text-body placeholder:text-text-faint"
  role="combobox"
  aria-autocomplete="list"
  aria-expanded={paths.length > 0}
  aria-controls="{uid}-list"
  aria-activedescendant={paths.length > 0 ? `${uid}-${String(cursor)}` : undefined}
  aria-labelledby={titleId}
  placeholder={say($lang, "finder_placeholder")}
  value={text}
  oninput={(event) => {
    text = event.currentTarget.value;
    cursor = 0;
  }}
  onkeydown={keys}
/>
<ul id="{uid}-list" class="max-h-palette overflow-y-auto" role="listbox" aria-labelledby={titleId}>
  {#each paths as path, at (path)}
    {@const drawn = parts(path)}
    <!-- svelte-ignore a11y_click_events_have_key_events (the box holds the focus and its keys open the active option, the APG Combobox pattern) -->
    <li
      id="{uid}-{String(at)}"
      role="option"
      aria-selected={at === cursor}
      class={[
        "flex cursor-pointer items-baseline gap-snug rounded-control px-base py-snug font-mono text-body",
        at === cursor ? "bg-page text-text" : "text-text-quiet hover:bg-page",
      ]}
      onmouseenter={() => {
        cursor = at;
      }}
      onclick={() => {
        open(path);
      }}
    >
      <span class="truncate">{drawn.name}</span>
      <span class="min-w-0 truncate text-note text-text-faint">{drawn.dir}</span>
    </li>
  {/each}
</ul>
{#if found !== undefined && found.text !== "" && paths.length === 0}
  <p class="px-snug text-note text-text-faint">{fill(say($lang, "finder_none"), { text: found.text })}</p>
{/if}
{#if found?.walked === "cut"}
  <p class="px-snug text-note text-text-faint">{say($lang, "finder_cut")}</p>
{/if}
