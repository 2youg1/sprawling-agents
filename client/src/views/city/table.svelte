<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The seat of the building table the city page opens on: the ledger
  // tree's first level, one row a building, with the counts a person
  // reads the city by - waiting for them, at work, done, the last start
  // - and one time bar per building on the runs board's folded clock. It
  // reads the language and hands `./table.ts`'s value to whatever
  // `./table.look.svelte` is.
  import { ui } from "../../ui";
  import type { Address, CityAnswer } from "../../wire";
  import type { BoardRun } from "../runs/lineage";
  import { lookOf } from "./table";
  import Look from "./table.look.svelte";

  interface Props {
    readonly city: CityAnswer;
    readonly runs: readonly BoardRun[];
    readonly now: number;
    readonly picked: Address | null;
    readonly onPick: (addr: Address) => void;
  }

  const { city, runs, now, picked, onPick }: Props = $props();
  const { lang } = ui();

  const look = $derived(
    lookOf({ city, runs, now, picked, lang: $lang }, (addr) => {
      onPick(addr);
    }),
  );
</script>

<Look {...look} />
