<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The seat of the harness cards: the official harnesses a subscription
  // reaches the city through, one card each (`crates/wire/Spec.lean`
  // §8-52).
  //
  // **The roster and the commands are the city's** (`Query::Harnesses`,
  // read from `agent_protocols::harness`); `harnesses.svelte` asks, this
  // seat hands the answer to `./harnesses` (`cardsOf`), and whatever
  // `./harness_cards.look.svelte` is draws it. Each card says what this
  // computer has of the harness, what command starts it, and where the
  // harness's own vendor says how to sign in. The state is one of three
  // (`harnessOf`): the launcher is missing, the harness is not installed
  // or not signed in - with the directories the city looked in - or it
  // is ready, with where it was found.
  import type { HarnessLine } from "../../wire";

  export interface HarnessCardsProps {
    readonly lines: readonly HarnessLine[];
  }
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Look from "./harness_cards.look.svelte";
  import { cardsOf } from "./harnesses";

  const { lines }: HarnessCardsProps = $props();
  const { lang } = ui();
  const cards = $derived(cardsOf(lines, $lang));
</script>

<Look {cards} />
