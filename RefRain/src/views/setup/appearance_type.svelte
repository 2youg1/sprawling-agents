<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
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
    faceStyle,
    stackRefused,
  } from "./appearance";
  import type { Axis, Setting } from "./appearance";
  import type { ReceiptLook } from "./card";
  import Card from "./card.look.svelte";
  import type { Choice } from "../parts/combobox";

  type Switch = "off" | "on";
  const SWITCHES: readonly Switch[] = ["off", "on"];
  const SWITCH_WORDS: Record<Switch, Key> = {
    off: "appearance_feature_off",
    on: "appearance_feature_on",
  };

  interface Props {
    readonly look: Appearance;
    readonly write: (next: Appearance, name: Setting) => void;
    readonly receipt: (name: Setting) => ReceiptLook;
  }

  const { look, write, receipt }: Props = $props();

  const lang = ui().lang;
  const said = (key: Key): string => say($lang, key);

  let box = $state("");
  let slot1 = $state("");
  let slot2 = $state("");
  let leading = $state("");
  let tracking = $state("");
  let installed = $state<readonly string[]>([]);
  let note = $state<string | undefined>(undefined);
  const localChoices: readonly Choice[] = $derived(
    installed.map((family) => ({ value: family, label: family, face: faceStyle(family) })),
  );

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

  const live: ReadonlySet<TypeFeature> = $derived(featuresOffered([look.sans, look.mono]));

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

  const wear = (axis: Axis, family: string): void => {
    write(
      axis === "sans"
        ? { ...look, sans: "custom", sansStack: family }
        : { ...look, mono: "custom", monoStack: family },
      axis === "sans" ? "face" : "mono",
    );
  };

  const wearFallback = (slot: 0 | 1, typed: string): void => {
    if (slot === 0) slot1 = typed;
    else slot2 = typed;
    const both = [slot === 0 ? typed : slot1, slot === 1 ? typed : slot2];
    if (!both.every((name) => name === "" || FAMILY_SHAPE.test(name))) return;
    const chain = both.filter((name) => name !== "");
    write({ ...look, fallbacks: chain }, "fallbacks");
    slot1 = chain[0] ?? "";
    slot2 = chain[1] ?? "";
  };

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

  const offered = (): boolean => typeof window.queryLocalFonts === "function";

  const list = (): void => {
    const ask = window.queryLocalFonts;
    if (ask === undefined) {
      note = say($lang, "appearance_local_none");
      return;
    }
    void ask()
      .then((faces) => {
        installed = [
          ...new Set(
            faces.map((face) => face.family).filter((family) => FAMILY_SHAPE.test(family)),
          ),
        ].sort((a, b) => a.localeCompare(b));
        note = undefined;
      })
      .catch(() => {
        note = say($lang, "appearance_local_denied");
      });
  };

  const sizeFloor = (): string =>
    fill(say($lang, "appearance_body_floor"), { min: String(BODY_PX.min) });

  const refusal = (stack: string): Pick<FieldProps, "error"> =>
    stackRefused(stack) ? { error: say($lang, "appearance_stack_refused") } : {};

  const sizeRefusal = (): Pick<FieldProps, "error"> =>
    sizingOf(box).kind === "refused" ? { error: sizeFloor() } : {};

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

{#snippet localAsk()}
  {#if offered()}
    {#if installed.length === 0}
      <Button label={say($lang, "appearance_local")} onPress={list} />
    {/if}
  {:else}
    <p class="text-note text-text-faint">{say($lang, "appearance_local_none")}</p>
  {/if}
  {#if note !== undefined}
    <p class="text-note text-alert" role="alert">{note}</p>
  {/if}
{/snippet}

{#snippet localPicker(label: string, held: string, onPick: (family: string) => void)}
  {#if localChoices.length > 0}
    <Combobox
      {label}
      placeholder={say($lang, "part_search")}
      empty={say($lang, "part_no_match")}
      choices={localChoices}
      value={held}
      {onPick}
    />
  {/if}
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
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render localAsk()}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render localPicker(
      fill(say($lang, "appearance_local_for"), { face: say($lang, name) }),
      axis === "sans" ? look.sansStack : look.monoStack,
      (family: string) => {
        wear(axis, family);
      },
    )}
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
</Card>

<Card
  title={say($lang, "appearance_fallbacks")}
  note={say($lang, "appearance_fallbacks_note")}
  constraint={say($lang, "appearance_fallbacks_help")}
  receipt={receipt("fallbacks")}
>
  <div class="flex flex-col gap-tight">
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render localAsk()}
    <Field
      label={say($lang, "appearance_fallback_first")}
      {...slotRefusal(slot1)}
      value={slot1}
      onInput={(typed: string) => {
        wearFallback(0, typed);
      }}
    />
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render localPicker(say($lang, "appearance_fallback_first"), slot1, (family: string) => {
      wearFallback(0, family);
    })}

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
    {#if slot1.trim() !== ""}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
      {@render localPicker(say($lang, "appearance_fallback_second"), slot2, (family: string) => {
        wearFallback(1, family);
      })}
    {/if}
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

<Card title={say($lang, "appearance_preview")} note={say($lang, "appearance_preview_note")}>
  <div class="flex flex-col gap-base rounded-card bg-chrome p-base">
    <div class="flex flex-col gap-tight">
      <span class="text-note text-text-faint">{say($lang, "appearance_face")}</span>
      <p class="font-sans text-body text-text">{say($lang, "appearance_sample")}</p>
    </div>
    <div class="flex flex-col gap-tight">
      <span class="text-note text-text-faint">{say($lang, "appearance_mono")}</span>
      <p class="font-mono text-body text-text-quiet">{say($lang, "appearance_sample")}</p>
    </div>
    <div class="flex flex-col gap-tight">
      <span class="text-note text-text-faint">{say($lang, "appearance_reading")}</span>
      <p class="font-read text-body leading-read text-text" data-read={look.reading}>
        {say($lang, "appearance_reading_sample")}
      </p>
    </div>
  </div>
</Card>
