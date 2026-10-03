<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The room in results-only mode: its runs by session, the newest
  // session first and the newest run first inside it, each drawn as its
  // result rather than as the turns that led there. The fold knows two
  // stretches of a room - the session now open and everything before it
  // - so those are the two groups; the open one is named by how it began,
  // and the earlier one folds behind its heading by the rule the whole
  // thread's divider reads (client D80).
  import Result from "./result.svelte";
  import type { Boundary } from "./forking";
  import { earlierDrawn } from "./earlier";
  import { fill, say } from "../../core/lang";
  import type { RunBelief } from "../../core/belief";
  import { ui } from "../../ui";

  interface Props {
    readonly shown: readonly RunBelief[];
    readonly earlier: readonly RunBelief[];
    readonly boundary: Boundary | null;
  }

  const { shown, earlier, boundary }: Props = $props();
  const { lang } = ui();

  let pressed = $state<boolean | null>(null);
  const open = $derived(pressed ?? earlierDrawn(shown.length, earlier.length) === "open");

  const heading = $derived(
    boundary?.kind === "forked"
      ? fill(say($lang, "results_session_forked"), {
          mother: boundary.mother.slice(0, 8),
          turn: String(boundary.turn),
        })
      : say($lang, "results_session_this"),
  );
  const earlierHeading = $derived(say($lang, "results_session_earlier"));
</script>

<p class="mb-wide text-note text-text-faint">
  {fill(say($lang, "results_room_counts"), {
    sessions: String((shown.length > 0 ? 1 : 0) + (earlier.length > 0 ? 1 : 0)),
    runs: String(shown.length + earlier.length),
  })}
</p>
{#if shown.length > 0}
  <section class="mb-section" aria-label={heading}>
    <h2 class="mb-base text-note text-text-quiet">{heading}</h2>
    <ul>
      {#each [...shown].reverse() as run (run.run)}
        <Result {run} />
      {/each}
    </ul>
  </section>
{/if}
{#if earlier.length > 0}
  <section class="mb-section" aria-label={earlierHeading}>
    <h2 class="mb-base text-note text-text-quiet">
      <button
        type="button"
        class="rounded-control px-tight hover:bg-chrome hover:text-text-quiet"
        aria-expanded={open}
        onclick={() => {
          pressed = !open;
        }}
      >
        {earlierHeading} · {open ? say($lang, "session_collapse") : say($lang, "session_expand")}
      </button>
    </h2>
    {#if open}
      <ul>
        {#each [...earlier].reverse() as run (run.run)}
          <Result {run} />
        {/each}
      </ul>
    {/if}
  </section>
{/if}
