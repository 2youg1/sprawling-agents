<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // Four quiet doors under the composer into the building's catalog and
  // its neighbours: tools, skills, MCP, and the session's worktree. The
  // counts that used to stand here (Always / On call) named no action a
  // hand could take, so the letters name the doors instead.
  import { say } from "../../core/lang";
  import { buildingOf } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import { focusCatalog } from "../building/catalog_focus.svelte";
  import type { CatalogFocus } from "../building/catalog_focus.svelte";

  interface Props {
    readonly room: Address;
  }

  const { room }: Props = $props();
  const u = ui();
  const { lang } = u;
  const building = $derived(buildingOf(room));

  const DOORS: readonly {
    readonly key: CatalogFocus;
    readonly letter: string;
    readonly word:
      "catalog_door_tools" | "catalog_door_skills" | "catalog_door_mcp" | "catalog_door_worktree";
  }[] = [
    { key: "tools", letter: "A", word: "catalog_door_tools" },
    { key: "skills", letter: "B", word: "catalog_door_skills" },
    { key: "mcp", letter: "C", word: "catalog_door_mcp" },
    { key: "worktree", letter: "D", word: "catalog_door_worktree" },
  ];

  function open(focus: CatalogFocus): void {
    // MCP and the worktree leave the catalog page, so a focus left for
    // them would stick until a later catalog visit and open the wrong pane.
    if (focus === "mcp") {
      u.go({ kind: "mcp" });
      return;
    }
    if (focus === "worktree") {
      u.go({ kind: "building", address: building });
      return;
    }
    focusCatalog(focus);
    u.go({ kind: "building", address: building, section: "catalog" });
  }
</script>

<span
  class="flex min-w-0 flex-wrap items-center gap-hair px-tight text-note text-text-faint"
  role="group"
  aria-label={say($lang, "bld_catalog")}
>
  {#each DOORS as door, at (door.key)}
    {#if at > 0}
      <!-- wording-ok: a visual separator between lettered doors, not a word -->
      <span aria-hidden="true" class="text-text-faint">/</span>
    {/if}
    <button
      type="button"
      class="inline-flex h-touch items-center gap-hair rounded-control px-tight transition-[background-color,color,scale] ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100"
      aria-label="{door.letter} {say($lang, door.word)}"
      onclick={() => {
        open(door.key);
      }}
    >
      <!-- wording-ok: the letter is the door's mnemonic key, same in every language -->
      <span class="font-mono text-text-quiet">{door.letter}</span>
      <span>{say($lang, door.word)}</span>
    </button>
  {/each}
</span>
