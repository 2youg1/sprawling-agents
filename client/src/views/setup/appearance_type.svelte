<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The type cards of the appearance group: the interface face, the mono
  // face, the reading face with a sample set in it, the fallback chain, the
  // body size, the three ratios that set a line, the features the bundled
  // faces carry, and a preview of the interface and mono faces side by
  // side.
  //
  // The record and its writing are the group's (`appearance.svelte`):
  // this file is handed the record as it stands, the one way to write a
  // change to it, and the receipt every card of the group ends with, so a
  // change made here lands through the same door, the same root element
  // and the same receipt as a change made on any other card. What it
  // owns is the state these cards alone have - the boxes mid-edit and the
  // faces this machine has installed.

  import { onMount } from "svelte";

  import type { Key } from "../../core/lang";
  import { fill, say } from "../../core/lang";
  import {
    FACES,
    FAMILY_SHAPE,
    FEATURES,
    READING_FACES,
    WEIGHTS,
    featuresOffered,
  } from "../../core/appearance";
  import type { Appearance, TypeFeature, Weight } from "../../core/appearance";
  import { LINE_HEIGHT_FLOOR, TRACKING_FLOOR, ratioOf, spelledRatio } from "../../core/metrics";
  import { sizingOf } from "../../core/sizing";
  import { BODY_PX } from "../../wire";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Combobox from "../parts/combobox.svelte";
  import Field from "../parts/field.svelte";
  import type { FieldProps } from "../parts/field.svelte";
  import Segmented from "../parts/segmented.svelte";
  import {
    FACE_WORDS,
    FEATURE_WORDS,
    READING_WORDS,
    WEIGHT_WORDS,
    cellsOf,
    drawnLeading,
    drawnSize,
    stackRefused,
  } from "./appearance";
  import type { Axis, Setting } from "./appearance";
  import type { ReceiptLook } from "./card";
  import Card from "./card.look.svelte";

  // The two states of one feature switch. A word of this card rather than
  // of the wire: the wire records which features are on, and a switch has
  // exactly these two postures.
  type Switch = "off" | "on";
  const SWITCHES: readonly Switch[] = ["off", "on"];
  const SWITCH_WORDS: Record<Switch, Key> = {
    off: "appearance_feature_off",
    on: "appearance_feature_on",
  };

  interface Props {
    readonly look: Appearance;
    readonly write: (next: Appearance, name: Setting) => void;
    // The receipt of one card, said when the last change landed on it.
    readonly receipt: (name: Setting) => ReceiptLook;
  }

  const { look, write, receipt }: Props = $props();

  const lang = ui().lang;
  const said = (key: Key): string => say($lang, key);

  // What is in the size box, which is not the size: a box mid-edit holds
  // text the page must not act on yet. The two fallback slots and the two
  // ratio boxes are the same: a name half typed or a number half typed is
  // text, not a value, so each keeps what was typed until it is one.
  let box = $state("");
  let slot1 = $state("");
  let slot2 = $state("");
  let leading = $state("");
  let tracking = $state("");
  // The faces this machine has, once a person asks for them and the
  // browser agrees. Empty until both happen.
  let installed = $state<readonly string[]>([]);
  let note = $state<string | undefined>(undefined);

  onMount(() => {
    box = look.body === null ? drawnSize(document.documentElement) : String(look.body);
    slot1 = look.fallbacks[0] ?? "";
    slot2 = look.fallbacks[1] ?? "";
    leading =
      look.lineHeight === null
        ? drawnLeading(document.documentElement)
        : spelledRatio(look.lineHeight);
    tracking = look.tracking === null ? "" : spelledRatio(look.tracking);
  });

  // The features the faces in use declare, drawn as one set: a switch one
  // of them declares moves that face's text, so it is live
  // (`featuresOffered`, `core/appearance.ts`).
  const live: ReadonlySet<TypeFeature> = $derived(featuresOffered([look.sans, look.mono]));

  // Every keystroke in the size box is read once, and only a whole
  // number from the floor up reaches the page.
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

  // One fallback slot. A name that is not a name is kept in the box and
  // not written: the field says why, and the chain keeps what it had.
  // An empty slot is dropped from the chain, so the second one only takes
  // a name once the first holds one.
  const wearFallback = (slot: 0 | 1, typed: string): void => {
    if (slot === 0) slot1 = typed;
    else slot2 = typed;
    const both = [slot === 0 ? typed : slot1, slot === 1 ? typed : slot2];
    if (!both.every((name) => name === "" || FAMILY_SHAPE.test(name))) return;
    const chain = both.filter((name) => name !== "");
    write({ ...look, fallbacks: chain }, "fallbacks");
    // A chain has no gaps, so emptying the first slot moves the second
    // name up: the boxes follow the record rather than showing a name in
    // a slot the record no longer keeps it in.
    slot1 = chain[0] ?? "";
    slot2 = chain[1] ?? "";
  };

  // The two ratio boxes, read the way the size box is: only a value at or
  // above the wire's floor reaches the page, and an empty box takes the
  // page back to the ratio the theme draws.
  const resizeLeading = (typed: string): void => {
    leading = typed;
    const said = ratioOf(typed, LINE_HEIGHT_FLOOR);
    switch (said.kind) {
      case "cleared":
        write({ ...look, lineHeight: null }, "metrics");
        return;
      case "set":
        write({ ...look, lineHeight: said.perMille }, "metrics");
        return;
      case "refused":
        return;
    }
  };

  const resizeTracking = (typed: string): void => {
    tracking = typed;
    const said = ratioOf(typed, TRACKING_FLOOR);
    switch (said.kind) {
      case "cleared":
        write({ ...look, tracking: null }, "metrics");
        return;
      case "set":
        write({ ...look, tracking: said.perMille }, "metrics");
        return;
      case "refused":
        return;
    }
  };

  // One switch. The set is written in the order `FEATURES` declares,
  // which is the order the file and the stylesheet read it in.
  const toggleFeature = (feature: TypeFeature, next: Switch): void => {
    const on = next === "on";
    write(
      {
        ...look,
        features: FEATURES.filter((each) => (each === feature ? on : look.features.includes(each))),
      },
      "features",
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
  const sizeFloor = (): string =>
    fill(say($lang, "appearance_body_floor"), { min: String(BODY_PX.min) });

  // A refusal, or no such property at all: spreading it keeps an absent
  // error apart from `error={undefined}`, which marks the box.
  const refusal = (stack: string): Pick<FieldProps, "error"> =>
    stackRefused(stack) ? { error: say($lang, "appearance_stack_refused") } : {};

  const sizeRefusal = (): Pick<FieldProps, "error"> =>
    sizingOf(box).kind === "refused" ? { error: sizeFloor() } : {};

  // A fallback slot's one line of constraint: a family name and nothing
  // that could splice a second one into the slot.
  const slotRefusal = (typed: string): Pick<FieldProps, "error"> =>
    typed !== "" && !FAMILY_SHAPE.test(typed)
      ? { error: say($lang, "appearance_fallback_refused") }
      : {};

  const leadingFloor = (): string =>
    fill(say($lang, "appearance_leading_floor"), {
      min: spelledRatio(LINE_HEIGHT_FLOOR),
    });

  const trackingFloor = (): string =>
    fill(say($lang, "appearance_tracking_floor"), {
      min: spelledRatio(TRACKING_FLOOR),
    });

  const leadingRefusal = (): Pick<FieldProps, "error"> =>
    ratioOf(leading, LINE_HEIGHT_FLOOR).kind === "refused" ? { error: leadingFloor() } : {};

  const trackingRefusal = (): Pick<FieldProps, "error"> =>
    ratioOf(tracking, TRACKING_FLOOR).kind === "refused" ? { error: trackingFloor() } : {};
</script>

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
      <!-- A machine has hundreds of faces, so they are searched rather
      than scrolled; the families are this machine's own words, never
      translated. -->
      <Combobox
        label={fill(say($lang, "appearance_local_for"), { face: say($lang, name) })}
        placeholder={say($lang, "part_search")}
        empty={say($lang, "part_no_match")}
        choices={installed.map((family) => ({ value: family, label: family }))}
        value={axis === "sans" ? look.sansStack : look.monoStack}
        onPick={(family: string) => {
          wear(axis, family);
        }}
      />
    {/if}
  </div>
{/snippet}

<Card
  title={say($lang, "appearance_face")}
  note={say($lang, "appearance_face_note")}
  receipt={receipt("face")}
>
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
</Card>

<Card
  title={say($lang, "appearance_mono")}
  note={say($lang, "appearance_mono_note")}
  receipt={receipt("mono")}
>
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
</Card>

<Card
  title={say($lang, "appearance_reading")}
  note={say($lang, "appearance_reading_note")}
  receipt={receipt("reading")}
>
  <Segmented
    label={say($lang, "appearance_reading")}
    options={cellsOf(READING_FACES, READING_WORDS, said)}
    held={look.reading}
    onPick={(reading) => {
      write({ ...look, reading }, "reading");
    }}
  />
  <!-- The sample carries the card's own choice, so it shows the face this card holds wherever the card is drawn. -->
  <p
    class="rounded-card bg-chrome p-base font-read text-body leading-read text-text"
    data-read={look.reading}
  >
    {say($lang, "appearance_reading_sample")}
  </p>
</Card>

<Card
  title={say($lang, "appearance_fallbacks")}
  note={say($lang, "appearance_fallbacks_note")}
  constraint={say($lang, "appearance_fallbacks_help")}
  receipt={receipt("fallbacks")}
>
  <div class="flex flex-col gap-tight">
    <Field
      label={say($lang, "appearance_fallback_first")}
      {...slotRefusal(slot1)}
      value={slot1}
      onInput={(typed: string) => {
        wearFallback(0, typed);
      }}
    />
    <!-- A chain with no gap: the second name is taken once the first
    holds one, so a name typed into the second slot is never read back in
    the first. -->
    <Field
      label={say($lang, "appearance_fallback_second")}
      {...slotRefusal(slot2)}
      help={slot1.trim() === ""
        ? say($lang, "appearance_fallback_needs_first")
        : say($lang, "appearance_fallback_second_help")}
      disabled={slot1.trim() === ""}
      value={slot2}
      onInput={(typed: string) => {
        wearFallback(1, typed);
      }}
    />
  </div>
</Card>

<Card
  title={say($lang, "appearance_body")}
  note={say($lang, "appearance_body_note")}
  constraint={sizeFloor()}
  receipt={receipt("body")}
>
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
</Card>

<Card
  title={say($lang, "appearance_metrics")}
  note={say($lang, "appearance_metrics_note")}
  receipt={receipt("metrics")}
>
  <div class="flex flex-col gap-base">
    <div class="flex flex-col gap-tight">
      <Field
        label={say($lang, "appearance_leading")}
        {...leadingRefusal()}
        value={leading}
        onInput={resizeLeading}
      />
      <p class="text-note text-text-faint">{leadingFloor()}</p>
    </div>
    <div class="flex flex-col gap-tight">
      <Field
        label={say($lang, "appearance_tracking")}
        suffix={say($lang, "appearance_tracking_unit")}
        {...trackingRefusal()}
        value={tracking}
        onInput={resizeTracking}
      />
      <p class="text-note text-text-faint">{trackingFloor()}</p>
    </div>
    <div class="flex min-w-0 flex-col gap-snug">
      <span class="text-note text-text-quiet">{say($lang, "appearance_weight")}</span>
      <Segmented
        label={say($lang, "appearance_weight")}
        options={cellsOf(WEIGHTS, WEIGHT_WORDS, said)}
        held={look.weight}
        onPick={(weight: Weight) => {
          write({ ...look, weight }, "metrics");
        }}
      />
      <p class="text-note text-text-faint">{say($lang, "appearance_weight_note")}</p>
    </div>
  </div>
</Card>

<Card
  title={say($lang, "appearance_features")}
  note={say($lang, "appearance_features_note")}
  receipt={receipt("features")}
>
  <div class="flex flex-col gap-snug">
    {#each FEATURES as feature (feature)}
      <!-- A switch the faces in use do not declare is left out of reach and
      says why, and its drawn posture is the one the page draws: off. -->
      {@const available = live.has(feature)}
      {@const absent = available ? {} : { why: say($lang, "appearance_feature_absent") }}
      <div class="flex flex-wrap items-center justify-between gap-base">
        <span class="text-note text-text-quiet">{say($lang, FEATURE_WORDS[feature])}</span>
        <Segmented
          label={say($lang, FEATURE_WORDS[feature])}
          options={cellsOf(SWITCHES, SWITCH_WORDS, said).map((cell) => ({ ...cell, ...absent }))}
          held={available && look.features.includes(feature) ? "on" : "off"}
          onPick={(next: Switch) => {
            toggleFeature(feature, next);
          }}
        />
      </div>
    {/each}
  </div>
</Card>

<Card title={say($lang, "appearance_preview")}>
  <div class="flex flex-col gap-tight rounded-card bg-chrome p-base">
    <p class="font-sans text-body text-text">{say($lang, "appearance_sample")}</p>
    <p class="font-mono text-body text-text-quiet">{say($lang, "appearance_sample")}</p>
  </div>
</Card>
