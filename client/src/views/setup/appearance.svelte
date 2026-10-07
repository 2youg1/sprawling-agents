<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the page is drawn: faces, body size, air, colour, motion, glass,
  // and how much of the world layer the blend tier shows.
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
  import { BLEND_PERCENT, CHROMAS, DENSITIES, FACES, GLASSES, LIGHTINGS, MOTIONS, blendOf } from "../../core/appearance";
  import { sizingOf } from "../../core/sizing";
  import { BODY_PX } from "../../wire";
  import type { Appearance } from "../../core/appearance";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import type { FieldProps } from "../parts/field.svelte";
  import Glyph from "../parts/glyph.svelte";
  import Segmented from "../parts/segmented.svelte";
  import Notifying from "./notifying.svelte";
  import Tier from "./tier.svelte";
  import Showing from "../shared/showing.svelte";
  import {
    CHROMA_WORDS,
    DENSITY_WORDS,
    FACE_WORDS,
    GLASS_WORDS,
    LIGHTING_WORDS,
    MOTION_WORDS,
    applyAppearance,
    cellsOf,
    drawnBlend,
    drawnSize,
    saveReceipt,
    stackRefused,
  } from "./appearance";
  import type { Axis } from "./appearance";

  // Which card a receipt belongs to: a setting, since one setting can
  // grow a second control (a face card carries its stack box).
  type Setting = "lighting" | "face" | "mono" | "body" | "density" | "chroma" | "motion" | "glass" | "blend";

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

  // What is in the size box, which is not the size: a box mid-edit holds
  // text the page must not act on yet.
  let box = $state("");
  // What the slider stands at: the person's figure, or the one the page
  // draws while they have stated none.
  let blend = $state<number | null>(null);
  // The faces this machine has, once a person asks for them and the
  // browser agrees. Empty until both happen.
  let installed = $state<readonly string[]>([]);
  let note = $state<string | undefined>(undefined);

  const write = (next: Appearance, name: Setting): void => {
    u.prefs.setAppearance(next);
    applyAppearance(root, next);
    receipt.landed(name);
  };

  // The screen can be reached before start-up applied the stored
  // choices, so it applies them itself; from then on `write` moves them.
  onMount(() => {
    applyAppearance(root, look);
    box = look.body === null ? drawnSize(root) : String(look.body);
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

  // Every keystroke in the size box is read once, and only a whole
  // number in range reaches the page.
  const resize = (typed: string): void => {
    box = typed;
    const saidSize = sizingOf(typed);
    switch (saidSize.kind) {
      case "cleared":
        write({ ...look, body: null }, "body");
        return;
      case "sized":
        write({ ...look, body: saidSize.px }, "body");
        return;
      case "refused":
        return;
    }
  };

  // A family from this machine pins its axis to `custom`; the axis is
  // the one the list is drawn under, never inferred.
  const wear = (axis: Axis, family: string): void => {
    write(
      axis === "sans"
        ? { ...look, sans: "custom", sansStack: family }
        : { ...look, mono: "custom", monoStack: family },
      axis === "sans" ? "face" : "mono",
    );
  };

  // Chromium lists this machine's faces after asking the person; any
  // other engine has no such door, and the text field is the fallback.
  const offered = (): boolean => typeof window.queryLocalFonts === "function";

  const list = (): void => {
    const ask = window.queryLocalFonts;
    if (ask === undefined) {
      note = say($lang, "appearance_local_none");
      return;
    }
    void ask()
      .then((faces) => {
        installed = [...new Set(faces.map((face) => face.family))].sort();
        note = undefined;
      })
      .catch(() => {
        note = say($lang, "appearance_local_denied");
      });
  };

  // The size box's one line of constraint: the card's foot carries it,
  // and a refused box repeats it where the person is looking.
  const sizeRange = (): string =>
    fill(say($lang, "appearance_body_range"), {
      min: String(BODY_PX.min),
      max: String(BODY_PX.max),
    });

  // A refusal, or no such property at all: spreading it keeps an absent
  // error apart from `error={undefined}`, which marks the box.
  const refusal = (stack: string): Pick<FieldProps, "error"> =>
    stackRefused(stack) ? { error: say($lang, "appearance_stack_refused") } : {};

  const sizeRefusal = (): Pick<FieldProps, "error"> =>
    sizingOf(box).kind === "refused" ? { error: sizeRange() } : {};
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

{#snippet stackFor(axis: Axis, name: Key)}
  <div class="flex flex-col gap-tight">
    <Field
      label={say($lang, "appearance_stack")}
      help={say($lang, "appearance_stack_help")}
      {...refusal(axis === "sans" ? look.sansStack : look.monoStack)}
      value={axis === "sans" ? look.sansStack : look.monoStack}
      mono
      onInput={(stack) => {
        write(
          axis === "sans" ? { ...look, sansStack: stack } : { ...look, monoStack: stack },
          axis === "sans" ? "face" : "mono",
        );
      }}
    />
    {#if offered()}
      <Button label={say($lang, "appearance_local")} onPress={list} />
    {:else}
      <p class="text-note text-text-faint">{say($lang, "appearance_local_none")}</p>
    {/if}
    {#if note !== undefined}
      <p class="text-note text-alert" role="alert">{note}</p>
    {/if}
    {#if installed.length > 0}
      <!-- One control tier and radius; the families are this machine's own words, never translated. -->
      <select
        class="h-control w-full min-w-0 rounded-control border border-edge-input bg-raised px-base text-body text-text"
        aria-label={fill(say($lang, "appearance_local_for"), { face: say($lang, name) })}
        onchange={(event) => {
          wear(axis, event.currentTarget.value);
        }}
      >
        {#each installed as family (family)}
          <option value={family}>{family}</option>
        {/each}
      </select>
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

  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_face")}</span>
    <p class="text-note text-text-faint">{say($lang, "appearance_face_note")}</p>
    <Segmented
      label={say($lang, "appearance_face")}
      options={cellsOf(FACES, FACE_WORDS, said)}
      held={look.sans}
      onPick={(sans) => {
        write({ ...look, sans }, "face");
      }}
    />
    {#if look.sans === "custom"}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
      {@render stackFor("sans", "appearance_face")}
    {/if}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render foot(undefined, "face")}
  </div>

  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_mono")}</span>
    <p class="text-note text-text-faint">{say($lang, "appearance_mono_note")}</p>
    <Segmented
      label={say($lang, "appearance_mono")}
      options={cellsOf(FACES, FACE_WORDS, said)}
      held={look.mono}
      onPick={(mono) => {
        write({ ...look, mono }, "mono");
      }}
    />
    {#if look.mono === "custom"}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
      {@render stackFor("mono", "appearance_mono")}
    {/if}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render foot(undefined, "mono")}
  </div>

  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_body")}</span>
    <p class="text-note text-text-faint">{say($lang, "appearance_body_note")}</p>
    <Field
      label={say($lang, "appearance_body")}
      labelling="hidden"
      {...sizeRefusal()}
      kind="number"
      step={1}
      suffix={say($lang, "appearance_body_unit")}
      mono
      value={box}
      onInput={resize}
    />
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render foot(sizeRange(), "body")}
  </div>

  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "appearance_preview")}</span>
    <div class="flex flex-col gap-tight rounded-card bg-chrome p-base">
      <p class="font-sans text-body text-text">{say($lang, "appearance_sample")}</p>
      <p class="font-mono text-body text-text-quiet">{say($lang, "appearance_sample")}</p>
    </div>
  </div>

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
