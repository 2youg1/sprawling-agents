<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the page is drawn: which face carries the prose, which carries
  // a value read character by character, how large a line of body text
  // is, how much air sits between the lines, how much colour the screen
  // takes, and whether anything moves.
  //
  // Every choice is a name; what each name means is `theme.css`, which
  // stays the one authority for a family, a size, a spacing step and a
  // colour. This view writes an attribute on the root element and reads
  // the result back in the same breath, so the page a person is looking
  // at *is* the preview.
  //
  // **A size the person has not stated is absent here too.** The body
  // size is the one appearance value that is a number rather than a name,
  // and an empty box removes the custom property instead of writing the
  // number the stylesheet already holds - so 15px has one home, in
  // `theme.css`, and this file cannot drift away from it.
  //
  // None of it is a fact about the city, so none of it goes over the
  // wire: like the language, it is part of the record `core/prefs.ts`
  // keeps and hands over, and this file names no stored row. What stays
  // here is the drawing: which attribute each choice becomes, and which
  // word it is offered under. What a choice means before it is drawn -
  // the word tables, the cells, the writing to the root element - is
  // `appearance.ts`, beside this file and read by `main.ts` too.
  //
  // Every setting below is one card (client-SPEC 4-36): a title, one
  // line saying what the setting governs, the control, and a foot that
  // shows the save receipt for a moment after an instant change lands
  // (ux-upgrades A2). None of these controls needs a submit, so no foot
  // on this screen carries a button.

  import { onMount } from "svelte";

  import type { Key } from "../../core/lang";
  import { fill, say } from "../../core/lang";
  import {
    BODY_PX,
    CHROMAS,
    DENSITIES,
    FACES,
    LIGHTINGS,
    MOTIONS,
    sizingOf,
  } from "../../core/prefs";
  import type { Appearance } from "../../core/prefs";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import type { FieldProps } from "../parts/field.svelte";
  import Glyph from "../parts/glyph.svelte";
  import Segmented from "../parts/segmented.svelte";
  import {
    CHROMA_WORDS,
    DENSITY_WORDS,
    FACE_WORDS,
    LIGHTING_WORDS,
    MOTION_WORDS,
    applyAppearance,
    cellsOf,
    drawnSize,
    saveReceipt,
    stackRefused,
  } from "./appearance";
  import type { Axis } from "./appearance";

  // Which card a receipt belongs to. Named for the setting rather than
  // the control, because one setting can grow a second control - the
  // face cards already carry the stack box beside their track.
  type Setting = "lighting" | "face" | "mono" | "body" | "density" | "chroma" | "motion";

  const u = ui();
  const lang = u.lang;
  const held = u.prefs.held;
  const root = document.documentElement;
  // Read through the door rather than copied into state here: a copy is
  // a second holder of one record, and the two part company the first
  // time anything else changes a preference.
  const look: Appearance = $derived($held.appearance);
  const said = (key: Key): string => say($lang, key);
  const receipt = saveReceipt();
  const saved = receipt.saved;

  // What is in the size box, which is not the size: a box mid-edit holds
  // text the page must not act on yet.
  let box = $state("");
  // The faces this machine has, once a person asks for them and the
  // browser agrees. Empty until both happen.
  let installed = $state<readonly string[]>([]);
  let note = $state<string | undefined>(undefined);

  const write = (next: Appearance, name: Setting): void => {
    u.prefs.setAppearance(next);
    applyAppearance(root, next);
    receipt.landed(name);
  };

  // The screen can be reached before whatever applies this at start-up
  // has run, so it puts the stored choices on the page itself; from then
  // on `write` is what moves them.
  onMount(() => {
    applyAppearance(root, look);
    box = look.body === null ? drawnSize(root) : String(look.body);
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

  // Choosing a family from this machine also pins the axis to `custom`,
  // so the choice holds however the list was reached; the axis is the
  // one the list is drawn under and is never inferred from which face
  // happens to be custom already.
  const wear = (axis: Axis, family: string): void => {
    write(
      axis === "sans"
        ? { ...look, sans: "custom", sansStack: family }
        : { ...look, mono: "custom", monoStack: family },
      axis === "sans" ? "face" : "mono",
    );
  };

  // Chromium answers with the faces this machine has installed, after
  // asking the person. Any other engine has no such door, and the text
  // field beside it is the whole fallback.
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

  // The one line of constraint on this screen's one numeric box. The
  // card's foot carries it (client-SPEC 4-36) and a refused box repeats
  // it as the field's own error, because a person who typed 99 needs it
  // where they are looking.
  const sizeRange = (): string =>
    fill(say($lang, "appearance_body_range"), {
      min: String(BODY_PX.min),
      max: String(BODY_PX.max),
    });

  // A refusal the field carries, or no such property at all. An absent
  // error and an error that is nothing are different states, and only
  // the first one leaves the box unmarked - which is what spreading the
  // property instead of passing `error={undefined}` keeps apart.
  const refusal = (stack: string): Pick<FieldProps, "error"> =>
    stackRefused(stack) ? { error: say($lang, "appearance_stack_refused") } : {};

  const sizeRefusal = (): Pick<FieldProps, "error"> =>
    sizingOf(box).kind === "refused" ? { error: sizeRange() } : {};
</script>

{#snippet foot(constraint: string | undefined, name: Setting)}
  <!-- The `{@render}` lines below each carry one suppression of
      `no-confusing-void-expression`, the way `parts/segmented.svelte`
      settles it. -->
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
      <!-- A15: one control tier and one control radius, like every
          other box on this screen. The families are this machine's own
          words and are never translated. -->
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

<div class="flex flex-col gap-base">
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
</div>
