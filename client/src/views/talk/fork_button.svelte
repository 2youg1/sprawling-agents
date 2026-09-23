<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The one branch action, revealed where a hand rests or a focus lands
(ux A7). It is named in words for a screen reader and drawn as the
branch mark for the eye.

`onHover` reports the entry under the hand - entered or focused, and
cleared as the hand leaves - which is the one the `fork.here` chord
branches from. Clearing matters: a chord pressed pages away must never
reach a stale entry. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { say } from "../../core/lang";
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

<button
  type="button"
  class="absolute top-0 right-0 rounded-control px-tight text-note text-text-faint opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100 focus:opacity-100 hover:bg-chrome hover:text-text-quiet"
  aria-label={say($lang, "fork_here")}
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
  <!-- wording-ok: a drawing in type, not a word; the action's name is the aria-label beside it. -->
  ⑂
</button>
