<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A hint's two placement branches, each drawn standing so the render
  // gate measures where it lands (client/Spec.lean §2, §4-18). The gate's
  // engine has anchor positioning, so `by-engine` is the anchor branch
  // there, and `against-wrapper` is the branch Safari and Firefox take,
  // drawn on its own. A hint at rest is `display: none` and nothing on
  // this route hovers, which is why the look is drawn here directly,
  // with `standing` in place of `wanted`.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import TipLook from "../../parts/tip.look.svelte";
  import { lookOf, type Placing, type TipSide } from "../../parts/tip";
  import Case from "../case.svelte";

  const { lang } = ui();
  const uid = $props.id();

  interface Placed {
    readonly label: string;
    readonly side: TipSide;
    readonly placing: Placing;
  }

  const PLACED: readonly Placed[] = [
    { label: "tip · above, against the anchor", side: "above", placing: "by-engine" },
    { label: "tip · above, against the wrapper", side: "above", placing: "against-wrapper" },
    { label: "tip · to the right, against the anchor", side: "right", placing: "by-engine" },
    { label: "tip · to the right, against the wrapper", side: "right", placing: "against-wrapper" },
  ];

  function standing(placed: Placed, at: number) {
    return {
      ...lookOf(
        { text: say($lang, "part_why_halted"), side: placed.side },
        { uid: `${uid}-${String(at)}`, dismissed: false, reengage: () => undefined },
      ),
      placing: placed.placing,
      showing: "standing" as const,
    };
  }
</script>

{#each PLACED as placed, at (placed.label)}
  <Case label={placed.label}>
    <!-- Room on every side, so the hint lands inside the fold whichever
    way the engine flips it. -->
    <div class="py-section">
      <TipLook {...standing(placed, at)}>
        {#snippet children(hint: string)}
          <button
            type="button"
            class="rounded-control px-base py-tight text-label text-text-quiet hover:bg-raised"
            aria-describedby={hint}
          >
            {say($lang, "part_save")}
          </button>
        {/snippet}
      </TipLook>
    </div>
  </Case>
{/each}
