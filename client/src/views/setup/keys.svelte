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

  // Keys that are only ever half of a chord.
  const MODIFIERS = ["Control", "Meta", "Shift", "Alt", "AltGraph", "CapsLock"];
</script>

<script lang="ts">
  import { ACTIONS, LABELS, keymap, spell } from "../../core/keys";
  import type { Action, Chord } from "../../core/keys";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import { Kbd } from "../parts/kbd.svelte";
  import Tip from "../parts/tip.svelte";

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

  function capture(action: Action, event: KeyboardEvent): void {
    if (MODIFIERS.includes(event.key)) {
      return;
    }
    event.preventDefault();
    if (event.key === "Escape") {
      recording = null;
      return;
    }
    // Shift is part of a chord only beside the accelerator: without
    // one, the browser already hands back the character the layout
    // produced, and judging shift as well would put `?` out of reach
    // on a keyboard that needs shift to type it.
    const accel = event.ctrlKey || event.metaKey;
    const chord: Chord = { accel, shift: accel && event.shiftKey, key: event.key };
    keys.bind(action, chord);
    recording = null;
  }
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
        <Tip text={say($lang, "keys_change")}>
          {#snippet children(hint: string)}
            <!-- This control carries what `parts/button.svelte` cannot:
                 a drawn chord while it waits and the listening posture
                 while it records. Enter and Space reach the one click
                 guard below through the platform, as every button's do
                 (client/Spec.lean §7-2). -->
            <button
              type="button"
              class="h-control-sm rounded-control border border-edge px-snug hover:bg-raised aria-pressed:bg-raised-hover"
              aria-describedby={hint}
              aria-pressed={recording === action}
              onclick={() => {
                recording = action;
              }}
              onblur={() => {
                recording = null;
              }}
              onkeydown={(event) => {
                if (recording === action) {
                  capture(action, event);
                }
              }}
            >
              {#if recording === action}
                <span class="font-mono text-note text-text-faint">{say($lang, "keys_press")}</span>
              {:else}
                <!-- The chord is redrawn whenever it changes: the marks
                     read `core/keys` outside the reactive surface, so
                     the key here is what brings them back. -->
                {#key spell($bound[action])}
                  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself, typed `void`; a snippet exported from another component resolves for `svelte-check` but not for the type-aware lint lane) -->
                  {@render Kbd({ action })}
                {/key}
              {/if}
            </button>
          {/snippet}
        </Tip>
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
