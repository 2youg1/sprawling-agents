<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script module lang="ts">
  export { MIN_WIDTH } from "./skyline";
</script>

<script lang="ts">
  import { heldWithin } from "../../core/belief/rooms";
  import { ui } from "../../ui";
  import type { Address, CityAnswer } from "../../wire";
  import { cornerPower } from "./shape";
  import { lookOf } from "./skyline";
  import Look from "./skyline.look.svelte";

  interface SkylineProps {
    readonly city: CityAnswer;
    readonly picked: Address | null;
    readonly onPick: (addr: Address) => void;
  }

  const { city, picked, onPick }: SkylineProps = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const power = cornerPower(document.documentElement);

  const look = $derived(
    lookOf(
      { city, picked, runsIn: (addr) => heldWithin($belief, addr), lang: $lang, power },
      (addr) => {
        onPick(addr);
      },
    ),
  );
</script>

<Look {...look} />
