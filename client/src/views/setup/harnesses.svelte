<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The harness page: asks the city for its harnesses and hands the
  // answer to the cards (`harness_cards.svelte`), which the gallery
  // draws from a fixture.
  import { QUERIES } from "../../core/asking";
  import { readAnswer } from "../../core/answered";
  import { ui } from "../../ui";
  import Unanswered from "../parts/unanswered.svelte";
  import HarnessCards from "./harness_cards.svelte";

  const u = ui();

  const asked = u.conn.asking.ask(QUERIES.harnesses);
  const read = $derived(readAnswer($asked, (held) => ("harnesses" in held ? held.harnesses.harnesses : undefined)));
</script>

{#if read.kind === "held"}
  <HarnessCards lines={read.value} />
{:else if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={QUERIES.harnesses} />
{/if}
