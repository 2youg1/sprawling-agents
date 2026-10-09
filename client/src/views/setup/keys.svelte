<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Where a key is changed. One row per action: what it does, the chord
  // that reaches it now, a button that listens for the next chord, and -
  // when two actions claim one chord - which other action it collides
  // with. Nothing is refused here; a collision is shown and left to the
  // person, because the chord they just pressed is the one they meant.
  //
  // This section is mounted by the settings page, which owns its own
  // column and its own section chrome; the heading below is the same one
  // that page draws so the section reads the same wherever it is placed.
  //
  // The button that records a chord is `chord.ts`, drawn by
  // `chord.look.svelte`; this file holds which row listens and the
  // keymap the chord lands in.
</script>

<script lang="ts">
  import { ACTIONS, LABELS, keymap, spell } from "../../core/keys";
  import type { Action } from "../../core/keys";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import { chordOf } from "./chord";
  import type { ChordHands } from "./chord";
  import ChordLook from "./chord.look.svelte";

  const { lang } = ui();
  const keys = keymap();
  const bound = keys.bound;
  const clashes = keys.conflicts;

  // The action listening for its new chord.
  let recording = $state<Action | null>(null);

  // The other actions claiming the chord this one reaches, if any. A
  // conflict is shown, not prevented: the person deciding which of the
  // two they meant needs to see both.
  function taken(action: Action): readonly Action[] {
    const clash = $clashes.find((held) => held.spelled === spell($bound[action]));
    return clash === undefined ? [] : clash.actions.filter((other) => other !== action);
  }

  const hands: ChordHands = {
    listen: (action) => {
      recording = action;
    },
    bind: (action, chord) => {
      keys.bind(action, chord);
    },
  };
</script>

<section class="border-t border-edge py-wide">
  <h2 class="mb-base text-label font-label text-text-quiet">{say($lang, "keys_title")}</h2>
  <ul class="flex flex-col">
    {#each ACTIONS as action (action)}
      <li class="flex items-center gap-base border-b border-edge py-snug text-label last:border-b-0">
        <span class="min-w-0 flex-1 truncate text-text-quiet">{say($lang, LABELS[action])}</span>
        {#if taken(action).length > 0}
          <span class="truncate text-note text-alert">
            {fill(say($lang, "keys_conflict"), { name: say($lang, LABELS[taken(action)[0] ?? action]) })}
          </span>
        {/if}
        <ChordLook
          {...chordOf(
            {
              action,
              spelled: spell($bound[action]),
              listening: recording === action,
              prompt: say($lang, "keys_press"),
              unbound: say($lang, "keys_unbound"),
              tip: say($lang, "keys_change"),
            },
            hands,
          )}
        />
        <!-- Rebuilt when the chord changes so that `keys.changed` - the
             one judge of whether this row left its default - is asked
             again at the moment the answer can differ. The reason rides
             a spread because a prop that is absent and a prop that is
             `undefined` are two different things to this typechecker. -->
        {#key spell($bound[action])}
          {@const stop = keys.changed(action) ? {} : { why: say($lang, "keys_at_default") }}
          <Button
            {...stop}
            label={say($lang, "keys_reset")}
            tone="quiet"
            onPress={() => {
              keys.reset(action);
            }}
          />
        {/key}
      </li>
    {/each}
  </ul>
  <Button
    label={say($lang, "keys_reset_all")}
    tone="quiet"
    onPress={() => {
      keys.resetAll();
    }}
  />
</section>
