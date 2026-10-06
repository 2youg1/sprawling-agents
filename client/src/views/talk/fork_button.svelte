<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The one branch action, revealed where a hand rests or a focus lands
(ux A7). Like every button (D55) it carries a mark the eye knows - the
branch glyph -, its name in words beside the mark, and a hint on hover
and focus that says what the branch keeps.

It stands at the end of its entry's own line - after the speaker and
time under a person's words, after the head of a reply - and never over
the words, because a button laid over text hides the text it would
branch from. It keeps its place while hidden, so revealing it moves
nothing. The nearest `group` around it is the entry it reveals with.

`onHover` reports the entry under the hand - entered or focused, and
cleared as the hand leaves - which is the one the `fork.here` chord
branches from. Clearing matters: a chord pressed pages away must never
reach a stale entry. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { say } from "../../core/lang";
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import { planFork } from "./forking";
  import type { ForkEntry, ForkPlan } from "./forking";
  import type { RunId } from "../../wire";

  interface Props {
    readonly run: RunId;
    readonly entry: ForkEntry;
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    readonly onHover: (entry: ForkEntry | null) => void;
  }

  const { run, entry, onFork, onHover }: Props = $props();

  const u = ui();
  const { lang } = u;
</script>

<div data-fork class="shrink-0 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100 focus-within:opacity-100">
  <Tip text={say($lang, "fork_here_hint")}>
    {#snippet children(hint: string)}
      <button
        type="button"
        class="flex items-center gap-tight rounded-control px-tight text-note text-text-faint hover:bg-chrome hover:text-text-quiet"
        aria-describedby={hint}
        onmouseenter={() => {
          onHover(entry);
        }}
        onmouseleave={() => {
          onHover(null);
        }}
        onfocus={() => {
          onHover(entry);
        }}
        onblur={() => {
          onHover(null);
        }}
        onclick={() => {
          onFork?.(planFork(run, entry));
        }}
      >
        <Glyph name="branch" size="sm" />
        {say($lang, "fork_here")}
      </button>
    {/snippet}
  </Tip>
</div>
