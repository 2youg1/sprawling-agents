<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // One call in full, drawn by the view its reading names (client/Spec.lean
  // §4-45): the patch it made, the terminal it printed to, the file it read
  // from, or the picture it took. `reading.ts` decides which; this file
  // only routes.
</script>

<script lang="ts">
  import { NO_TAIL } from "../../core/live_output";
  import { ui } from "../../ui";
  import type { Address, Call, RoundsAnswer } from "../../wire";
  import Diff from "./diff.svelte";
  import File from "./file.svelte";
  import { pictureOf, readFrom, readingOf } from "./reading";
  import Shot from "./shot.svelte";
  import Terminal from "./terminal.svelte";

  interface Props {
    readonly call: Call;
    readonly rounds: RoundsAnswer;
    readonly talk: Address;
  }

  const { call, rounds, talk }: Props = $props();

  const live = ui().conn.live;

  const reading = $derived(readingOf(call));
  const read = $derived(reading === "file" ? readFrom(call) : null);
  const picture = $derived(reading === "shot" ? pictureOf(call) : null);
</script>

{#if reading === "diff"}
  <Diff {call} {rounds} {talk} />
{:else if read !== null}
  <File {read} />
{:else if picture !== null}
  <Shot {picture} />
{:else}
  <Terminal {call} tail={$live[rounds.run] ?? NO_TAIL} />
{/if}
