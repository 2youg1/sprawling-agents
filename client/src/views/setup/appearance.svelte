<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the page is drawn: faces, body size, air, colour, motion, glass,
  // and how much of the world layer the blend tier shows. The cards for
  // the faces and the size are `appearance_type.svelte`, which writes
  // through `write` and ends each card with `foot` like the rest.
  //
  // Every choice is a name or a number whose meaning is the theme's.
  // This view writes the root element and the page a person is looking
  // at *is* the preview. **A number the person has not stated is absent
  // here too**: an empty size box or an untouched slider removes the
  // custom property instead of writing the stylesheet's own figure, so
  // that figure keeps its one home. The record is `core/prefs.ts`'s and
  // this file names no stored row; the word tables and the writing to
  // the root element are `appearance.ts`, which `main.ts` reads too.
  //
  // Every setting is one card (client/Spec.lean §4-36): title, one line, the
  // control, and a foot with the save receipt; nothing needs a submit.

  import { onMount } from "svelte";

  import type { Key } from "../../core/lang";
  import { fill, say } from "../../core/lang";
  import { BLEND_PERCENT, CHROMAS, DENSITIES, GLASSES, LIGHTINGS, MOTIONS, blendOf } from "../../core/appearance";
  import type { Appearance } from "../../core/appearance";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import Segmented from "../parts/segmented.svelte";
  import AppearanceType from "./appearance_type.svelte";
  import Notifying from "./notifying.svelte";
  import Tier from "./tier.svelte";
  import Showing from "../shared/showing.svelte";
  import {
    CHROMA_WORDS,
    DENSITY_WORDS,
    GLASS_WORDS,
    LIGHTING_WORDS,
    MOTION_WORDS,
    applyAppearance,
    cellsOf,
    drawnBlend,
    saveReceipt,
  } from "./appearance";
  import type { Setting } from "./appearance";

  const u = ui();
  const lang = u.lang;
  const held = u.prefs.held;
  const root = document.documentElement;
  // Read through the door, never copied: a copy is a second holder of
  // one record.
  const look: Appearance = $derived($held.appearance);
  const said = (key: Key): string => say($lang, key);
  const receipt = saveReceipt();
  const saved = receipt.saved;

  // What the slider stands at: the person's figure, or the one the page
  // draws while they have stated none.
  let blend = $state<number | null>(null);

  const write = (next: Appearance, name: Setting): void => {
    u.prefs.setAppearance(next);
    applyAppearance(root, next);
    receipt.landed(name);
  };

  // The screen can be reached before start-up applied the stored
  // choices, so it applies them itself; from then on `write` moves them.
  onMount(() => {
    applyAppearance(root, look);
    blend = look.blend ?? drawnBlend(root);
  });

  const reblend = (moved: string): void => {
    const percent = blendOf(moved);
    if (percent === null) return;
    blend = percent;
    write({ ...look, blend: percent }, "blend");
  };

  const blendRange = (): string =>
    fill(say($lang, "appearance_blend_range"), {
      min: String(BLEND_PERCENT.min),
      max: String(BLEND_PERCENT.max),
      step: String(BLEND_PERCENT.step),
    });
</script>

{#snippet foot(constraint: string | undefined, name: Setting)}
  <!-- Each `{@render}` below carries one lint suppression, as `parts/segmented.svelte` does. -->
  <div class="mt-tight flex items-baseline justify-between gap-base">
    {#if constraint !== undefined}
      <span class="text-note text-text-faint">{constraint}</span>
    {/if}
    {#if $saved === name}
      <span class="fade ml-auto inline-flex items-center gap-tight text-note text-text-quiet">
        <Glyph name="check" size="sm" class="shrink-0" />
        {say($lang, "setup_saved")}
      </span>
    {/if}
  </div>
{/snippet}

<!-- The cards flow into the grid of the group that holds them, so the
language card the settings page adds beside them takes the next cell. -->
<div class="contents">
  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_lighting")}</span>
    <p class="text-note text-text-faint">{say($lang, "appearance_lighting_note")}</p>
    <Segmented
      label={say($lang, "appearance_lighting")}
      options={cellsOf(LIGHTINGS, LIGHTING_WORDS, said)}
      held={look.lighting}
      onPick={(lighting) => {
        write({ ...look, lighting }, "lighting");
      }}
    />
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render foot(undefined, "lighting")}
  </div>

  <AppearanceType {look} {write} {foot} />

  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_density")}</span>
    <p class="text-note text-text-faint">{say($lang, "appearance_density_note")}</p>
    <Segmented
      label={say($lang, "appearance_density")}
      options={cellsOf(DENSITIES, DENSITY_WORDS, said)}
      held={look.density}
      onPick={(density) => {
        write({ ...look, density }, "density");
      }}
    />
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render foot(undefined, "density")}
  </div>

  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_chroma")}</span>
    <p class="text-note text-text-faint">{say($lang, "appearance_chroma_note")}</p>
    <Segmented
      label={say($lang, "appearance_chroma")}
      options={cellsOf(CHROMAS, CHROMA_WORDS, said)}
      held={look.chroma}
      onPick={(chroma) => {
        write({ ...look, chroma }, "chroma");
      }}
    />
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render foot(undefined, "chroma")}
  </div>

  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_motion")}</span>
    <p class="text-note text-text-faint">{say($lang, "appearance_motion_note")}</p>
    <Segmented
      label={say($lang, "appearance_motion")}
      options={cellsOf(MOTIONS, MOTION_WORDS, said)}
      held={look.motion}
      onPick={(motion) => {
        write({ ...look, motion }, "motion");
      }}
    />
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render foot(undefined, "motion")}
  </div>
  <Tier />
  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_glass")}</span>
    <p class="text-note text-text-faint">{say($lang, "appearance_glass_note")}</p>
    <Segmented
      label={say($lang, "appearance_glass")}
      options={cellsOf(GLASSES, GLASS_WORDS, said)}
      held={look.glass}
      onPick={(glass) => {
        write({ ...look, glass }, "glass");
      }}
    />
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render foot(undefined, "glass")}
  </div>

  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_blend")}</span>
    <p class="text-note text-text-faint">{say($lang, "appearance_blend_note")}</p>
    <div class="flex h-control items-center gap-base">
      <input
        type="range"
        class="min-w-0 flex-1"
        min={BLEND_PERCENT.min}
        max={BLEND_PERCENT.max}
        step={BLEND_PERCENT.step}
        value={blend ?? BLEND_PERCENT.min}
        aria-label={say($lang, "appearance_blend")}
        aria-valuetext={blend === null ? undefined : `${String(blend)}%`}
        oninput={(event) => {
          reblend(event.currentTarget.value);
        }}
      />
      <span class="figure w-figure text-right text-body text-text">{blend === null ? "" : `${String(blend)}%`}</span>
    </div>
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render foot(blendRange(), "blend")}
  </div>
  <Notifying />

  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "showing_label")}</span>
    <p class="text-note text-text-faint">{say($lang, "showing_note")}</p>
    <Showing />
  </div>
</div>
