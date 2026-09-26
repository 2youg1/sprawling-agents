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
  // - so those are the two groups; the open one is named by how it began.
  import Result from "./result.svelte";
  import type { Boundary } from "./forking";
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

  const sessions = $derived(
    [
      {
        key: "open",
        heading:
          boundary?.kind === "forked"
            ? fill(say($lang, "results_session_forked"), {
                mother: boundary.mother.slice(0, 8),
                turn: String(boundary.turn),
              })
            : say($lang, "results_session_this"),
        runs: [...shown].reverse(),
      },
      { key: "earlier", heading: say($lang, "results_session_earlier"), runs: [...earlier].reverse() },
    ].filter((session) => session.runs.length > 0),
  );
</script>

<p class="mb-wide text-note text-text-faint">
  {fill(say($lang, "results_room_counts"), {
    sessions: String(sessions.length),
    runs: String(shown.length + earlier.length),
  })}
</p>
{#each sessions as session (session.key)}
  <section class="mb-section" aria-label={session.heading}>
    <h2 class="mb-base text-note text-text-quiet">{session.heading}</h2>
    <ul>
      {#each session.runs as run (run.run)}
        <Result {run} />
      {/each}
    </ul>
  </section>
{/each}
