<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the page is drawn: faces, body size, air, colour, motion, glass,
  // and how much of the world layer the blend tier shows. The cards for
  // the faces and the size are `appearance_type.svelte`, which writes
  // through `write` and gives each card its `receipt` like the rest.
  // Every card is drawn by `card.look.svelte` and the blend slider by
  // `slider.look.svelte`.
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
  import Segmented from "../parts/segmented.svelte";
  import AppearanceType from "./appearance_type.svelte";
  import { receiptOf } from "./card";
  import type { ReceiptLook } from "./card";
  import Card from "./card.look.svelte";
  import { sliderOf } from "./slider";
  import Slider from "./slider.look.svelte";
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
  const landing = saveReceipt();
  const saved = landing.saved;

  // What the slider stands at: the person's figure, or the one the page
  // draws while they have stated none.
  let blend = $state<number | null>(null);

  const write = (next: Appearance, name: Setting): void => {
    u.prefs.setAppearance(next);
    applyAppearance(root, next);
    landing.landed(name);
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

  const receipt = (name: Setting): ReceiptLook => receiptOf($saved === name, say($lang, "setup_saved"));

  const blendRange = (): string =>
    fill(say($lang, "appearance_blend_range"), {
      min: String(BLEND_PERCENT.min),
      max: String(BLEND_PERCENT.max),
      step: String(BLEND_PERCENT.step),
    });
</script>

<!-- The cards flow into the grid of the group that holds them, so the
language card the settings page adds beside them takes the next cell. -->
<div class="contents">
  <Card title={say($lang, "appearance_lighting")} note={say($lang, "appearance_lighting_note")} receipt={receipt("lighting")}>
    <Segmented
      label={say($lang, "appearance_lighting")}
      options={cellsOf(LIGHTINGS, LIGHTING_WORDS, said)}
      held={look.lighting}
      onPick={(lighting) => {
        write({ ...look, lighting }, "lighting");
      }}
    />
  </Card>

  <AppearanceType {look} {write} {receipt} />

  <Card title={say($lang, "appearance_density")} note={say($lang, "appearance_density_note")} receipt={receipt("density")}>
    <Segmented
      label={say($lang, "appearance_density")}
      options={cellsOf(DENSITIES, DENSITY_WORDS, said)}
      held={look.density}
      onPick={(density) => {
        write({ ...look, density }, "density");
      }}
    />
  </Card>

  <Card title={say($lang, "appearance_chroma")} note={say($lang, "appearance_chroma_note")} receipt={receipt("chroma")}>
    <Segmented
      label={say($lang, "appearance_chroma")}
      options={cellsOf(CHROMAS, CHROMA_WORDS, said)}
      held={look.chroma}
      onPick={(chroma) => {
        write({ ...look, chroma }, "chroma");
      }}
    />
  </Card>

  <Card title={say($lang, "appearance_motion")} note={say($lang, "appearance_motion_note")} receipt={receipt("motion")}>
    <Segmented
      label={say($lang, "appearance_motion")}
      options={cellsOf(MOTIONS, MOTION_WORDS, said)}
      held={look.motion}
      onPick={(motion) => {
        write({ ...look, motion }, "motion");
      }}
    />
  </Card>

  <Tier />

  <Card title={say($lang, "appearance_glass")} note={say($lang, "appearance_glass_note")} receipt={receipt("glass")}>
    <Segmented
      label={say($lang, "appearance_glass")}
      options={cellsOf(GLASSES, GLASS_WORDS, said)}
      held={look.glass}
      onPick={(glass) => {
        write({ ...look, glass }, "glass");
      }}
    />
  </Card>

  <Card title={say($lang, "appearance_blend")} note={say($lang, "appearance_blend_note")} constraint={blendRange()} receipt={receipt("blend")}>
    <Slider
      {...sliderOf({
        label: say($lang, "appearance_blend"),
        range: BLEND_PERCENT,
        at: blend,
        figure: (percent) => `${String(percent)}%`,
        onMove: reblend,
      })}
    />
  </Card>

  <Notifying />

  <Card title={say($lang, "showing_label")} note={say($lang, "showing_note")}>
    <Showing />
  </Card>
</div>
