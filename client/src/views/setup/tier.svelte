<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The tier, chosen in the appearance group (refrain roadmap 3-6, 3-8):
  // the same three the layers key cycles, as one radio group a finger
  // can press, writing the same preference. Nothing here is the tier's
  // own: the names and their order are `core/prefs.ts`'s and the words
  // the layers key's.
  import { say } from "../../core/lang";
  import { TIERS } from "../../core/prefs";
  import type { Tier } from "../../core/prefs";
  import { ui } from "../../ui";
  import Segmented from "../parts/segmented.svelte";
  import Card from "./card.look.svelte";

  const u = ui();
  const held = u.prefs.held;
  const lang = u.lang;

  function pick(tier: Tier): void {
    u.prefs.setTier(tier);
  }
</script>

<Card title={say($lang, "appearance_tier")} note={say($lang, "appearance_tier_note")}>
  <Segmented
    label={say($lang, "appearance_tier")}
    options={TIERS.map((each) => ({ value: each, label: say($lang, `tier_${each}`) }))}
    held={$held.tier}
    onPick={pick}
  />
</Card>
