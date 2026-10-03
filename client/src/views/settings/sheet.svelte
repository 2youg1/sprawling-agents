<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What the settings panel holds (client/Spec.lean §7L): the tree in a column
  // of its own on the left, under the panel's name and its close key,
  // and the group the tree points at beside it. The two scroll apart, so
  // a long group never scrolls the tree out of reach; a panel narrower
  // than `lg` stacks them and scrolls as one. Drawn inside the
  // panel's `<dialog>` and, on its own, by the gallery, which cannot
  // open a modal without covering every other specimen.

  import { say } from "../../core/lang";
  import type { SetupGroup, View } from "../../core/route";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import Setup from "../setup.svelte";
  import Tree from "./tree.svelte";

  interface Props {
    readonly group: SetupGroup;
    readonly beneath: View;
    readonly onPick: (group: SetupGroup) => void;
    readonly onClose: () => void;
    // The id the group's heading carries, which names the panel.
    readonly titleId: string;
  }

  const { group, beneath, onPick, onClose, titleId }: Props = $props();
  const { lang } = ui();

  let body = $state<HTMLElement | undefined>(undefined);

  // A group picked from the tree starts at its own top.
  let drawn: SetupGroup | null = null;
  $effect(() => {
    if (group === drawn) return;
    drawn = group;
    body?.scrollTo({ top: 0 });
  });
</script>

<!-- A container cannot answer queries about itself, so the frame that is
  measured and the row that is laid out are two boxes. The tree is as
  wide as the shell's left silver part, less half a gutter, so the group
  beside it starts on the line the conversation starts on (client D24). -->
<div class="@container/sheet h-full min-h-0">
<div class="flex h-full min-h-0 @max-lg/sheet:flex-col @max-lg/sheet:overflow-y-auto">
  <div
    class="flex w-[calc(var(--silver-side)-var(--spacing-gutter)/2)] shrink-0 flex-col gap-base overflow-y-auto border-r border-edge px-base py-wide @max-lg/sheet:w-full @max-lg/sheet:overflow-visible @max-lg/sheet:border-r-0 @max-lg/sheet:border-b @max-lg/sheet:py-base"
  >
    <div class="flex items-center justify-between px-snug">
      <span class="text-label font-label text-text">{say($lang, "nav_settings")}</span>
      <button
        type="button"
        class="-mr-tight flex size-control-sm items-center justify-center rounded-control text-text-faint hover:bg-raised hover:text-text"
        aria-label={say($lang, "settings_close")}
        onclick={onClose}
      >
        <Glyph name="cross" size="sm" />
      </button>
    </div>
    <Tree {group} {beneath} {onPick} />
  </div>
  <!-- The group is the panel's second level: it arrives half a panel
    later than the panel (`slide-then`, docs/frontend-method.md §4-43). -->
  <section
    bind:this={body}
    class="slide-then @container/page flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto @max-lg/sheet:overflow-visible"
    aria-labelledby={titleId}
  >
    <!-- Each group picked arrives with a short shift (docs/frontend-method.md §4-43). -->
    {#key group}
      <div class="shift">
        <Setup {group} {titleId} />
      </div>
    {/key}
  </section>
</div>
</div>
