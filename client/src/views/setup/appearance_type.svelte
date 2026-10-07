<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The type cards of the appearance group: the interface face, the
  // mono face, the reading face with a sample set in it, the body size,
  // and a preview of the interface and mono faces side by side.
  //
  // The record and its writing are the group's (`appearance.svelte`):
  // this file is handed the record as it stands, the one way to write a
  // change to it, and the receipt every card of the group ends with, so a
  // change made here lands through the same door, the same root element
  // and the same receipt as a change made on any other card. What it
  // owns is the state these cards alone have - the size box mid-edit and
  // the faces this machine has installed.

  import { onMount } from "svelte";

  import type { Key } from "../../core/lang";
  import { fill, say } from "../../core/lang";
  import { FACES, READING_FACES } from "../../core/appearance";
  import type { Appearance } from "../../core/appearance";
  import { sizingOf } from "../../core/sizing";
  import { BODY_PX } from "../../wire";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Combobox from "../parts/combobox.svelte";
  import Field from "../parts/field.svelte";
  import type { FieldProps } from "../parts/field.svelte";
  import Segmented from "../parts/segmented.svelte";
  import { FACE_WORDS, READING_WORDS, cellsOf, drawnSize, stackRefused } from "./appearance";
  import type { Axis, Setting } from "./appearance";
  import type { ReceiptLook } from "./card";
  import Card from "./card.look.svelte";

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
  // text the page must not act on yet.
  let box = $state("");
  // The faces this machine has, once a person asks for them and the
  // browser agrees. Empty until both happen.
  let installed = $state<readonly string[]>([]);
  let note = $state<string | undefined>(undefined);

  onMount(() => {
    box = look.body === null ? drawnSize(document.documentElement) : String(look.body);
  });

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

<Card title={say($lang, "appearance_face")} note={say($lang, "appearance_face_note")} receipt={receipt("face")}>
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

<Card title={say($lang, "appearance_mono")} note={say($lang, "appearance_mono_note")} receipt={receipt("mono")}>
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

<Card title={say($lang, "appearance_reading")} note={say($lang, "appearance_reading_note")} receipt={receipt("reading")}>
  <Segmented
    label={say($lang, "appearance_reading")}
    options={cellsOf(READING_FACES, READING_WORDS, said)}
    held={look.reading}
    onPick={(reading) => {
      write({ ...look, reading }, "reading");
    }}
  />
  <!-- The sample carries the card's own choice, so it shows the face this card holds wherever the card is drawn. -->
  <p class="rounded-card bg-chrome p-base font-read text-body leading-relaxed text-text" data-read={look.reading}>
    {say($lang, "appearance_reading_sample")}
  </p>
</Card>

<Card title={say($lang, "appearance_body")} note={say($lang, "appearance_body_note")} constraint={sizeFloor()} receipt={receipt("body")}>
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

<Card title={say($lang, "appearance_preview")}>
  <div class="flex flex-col gap-tight rounded-card bg-chrome p-base">
    <p class="font-sans text-body text-text">{say($lang, "appearance_sample")}</p>
    <p class="font-mono text-body text-text-quiet">{say($lang, "appearance_sample")}</p>
  </div>
</Card>
