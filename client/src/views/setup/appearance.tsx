// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the page is drawn: which face carries the prose, which carries a
// value read character by character, how large a line of body text is,
// how much air sits between the lines, how much colour the screen
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
// word it is offered under.

import { For, Show, createSignal, onMount } from "solid-js";

import type { Key } from "../../core/lang";
import {
  BODY_PX,
  CHROMAS,
  DENSITIES,
  FACES,
  LIGHTINGS,
  MOTIONS,
  STACK_SHAPE,
  preferences,
  sizingOf,
} from "../../core/prefs";
import type { Appearance, Chroma, Density, Face, Lighting, Motion, PreferenceDoor } from "../../core/prefs";
import { useSay } from "../../ui";
import { Field } from "../parts/field";
import type { FieldProps } from "../parts/field";
import { Button } from "../parts/button";
import { Segmented } from "../parts/segmented";

// The Local Font Access API, which Chromium offers and other engines do
// not. Declared optional so the capability check is the type check.
declare global {
  interface Window {
    queryLocalFonts?: () => Promise<readonly { readonly family: string }[]>;
  }
}

// Which of the two axes a face applies to. Named because the list of
// installed faces writes to the axis it is drawn under, and an axis
// inferred from which face happens to be custom made one of the two
// unreachable whenever both were.
type Axis = "sans" | "mono";

// The custom property the body size is written to, which is the same
// property `theme.css` declares the default in and derives the note and
// label steps from.
const BODY_PROPERTY = "--text-body";

// The word each stored value is offered under. A record keyed by the
// value rather than a list of pairs: the set of values belongs to
// `core/prefs.ts`, which reads every stored row back through it, and a
// value added there leaves this table refusing to compile until
// somebody gives it a word.
const LIGHTING_WORDS: Record<Lighting, Key> = {
  system: "appearance_lighting_system",
  dark: "appearance_lighting_dark",
  light: "appearance_lighting_light",
};
const FACE_WORDS: Record<Face, Key> = {
  geist: "appearance_face_geist",
  system: "appearance_face_system",
  custom: "appearance_face_custom",
};
const DENSITY_WORDS: Record<Density, Key> = {
  comfortable: "appearance_density_comfortable",
  compact: "appearance_density_compact",
};
const CHROMA_WORDS: Record<Chroma, Key> = {
  full: "appearance_chroma_full",
  off: "appearance_chroma_none",
};
const MOTION_WORDS: Record<Motion, Key> = {
  system: "appearance_motion_system",
  on: "appearance_motion_full",
  off: "appearance_motion_off",
};

// One control's cells: every value this build can load, in the order
// `core/prefs.ts` offers them, each under the word this file gives it.
function cellsOf<T extends string>(
  offered: readonly T[],
  words: Record<T, Key>,
  say: (key: Key) => string,
): readonly { readonly value: T; readonly label: string }[] {
  return offered.map((each) => ({ value: each, label: say(words[each]) }));
}

// One face, as the root element spells it: an attribute naming the
// choice, and - for a stack somebody typed - the stack itself, which is
// the one value the stylesheet cannot hold in advance.
function wearFace(root: HTMLElement, axis: Axis, face: Face, stack: string): void {
  const property = axis === "sans" ? "--face-sans" : "--face-mono";
  root.dataset[axis] = face;
  if (face === "custom" && STACK_SHAPE.test(stack)) {
    root.style.setProperty(property, stack);
  } else {
    root.style.removeProperty(property);
  }
}

// Which way the machine says it is lit. The query is the only one a
// browser offers, so "not light" is what dark means here.
const MACHINE_LIGHT = "(prefers-color-scheme: light)";

function machineLighting(): "dark" | "light" {
  return window.matchMedia(MACHINE_LIGHT).matches ? "light" : "dark";
}

// Follow the machine while nobody has said otherwise. Called once, from
// the one place that starts the client: a listener added wherever the
// settings screen mounts would be a second listener doing the same
// work, and neither would know about the other.
export function watchMachineLighting(root: HTMLElement, kept: PreferenceDoor): void {
  window.matchMedia(MACHINE_LIGHT).addEventListener("change", () => {
    const held = kept.held().appearance;
    if (held.lighting === "system") applyAppearance(root, held);
  });
}

export function applyAppearance(root: HTMLElement, held: Appearance): void {
  root.dataset.theme = held.lighting === "system" ? machineLighting() : held.lighting;
  wearFace(root, "sans", held.sans, held.sansStack);
  wearFace(root, "mono", held.mono, held.monoStack);
  if (held.body === null) {
    root.style.removeProperty(BODY_PROPERTY);
  } else {
    root.style.setProperty(BODY_PROPERTY, `${String(held.body)}px`);
  }
  root.dataset.density = held.density;
  root.dataset.chroma = held.chroma;
  root.dataset.motion = held.motion;
}

// The size the page is drawn at right now, in whole pixels, read back
// off the root element. A box nobody has filled in shows what the
// person is looking at, and this file never repeats the stylesheet's
// own figure.
function drawnSize(root: HTMLElement): string {
  const drawn = window.getComputedStyle(root).getPropertyValue(BODY_PROPERTY);
  return /^([0-9]+)/.exec(drawn.trim())?.[1] ?? "";
}

export function AppearanceSection() {
  const say = useSay();
  const root = document.documentElement;
  const kept = preferences();
  // Read through the door rather than copied into a signal here: a
  // copy is a second holder of one record, and the two part company
  // the first time anything else changes a preference.
  const held = (): Appearance => kept.held().appearance;
  // What is in the size box, which is not the size: a box mid-edit
  // holds text the page must not act on yet.
  const [box, setBox] = createSignal("");
  // The faces this machine has, once a person asks for them and the
  // browser agrees. Empty until both happen.
  const [installed, setInstalled] = createSignal<readonly string[]>([]);
  const [note, setNote] = createSignal<string | undefined>(undefined);

  const write = (next: Appearance) => {
    kept.setAppearance(next);
    applyAppearance(root, next);
  };

  // The screen can be reached before whatever applies this at start-up
  // has run, so it puts the stored choices on the page itself; from
  // then on `write` is what moves them.
  onMount(() => {
    applyAppearance(root, held());
    const stored = held().body;
    setBox(stored === null ? drawnSize(root) : String(stored));
  });

  // Every keystroke in the size box is read once, and only a whole
  // number in range reaches the page.
  const resize = (typed: string) => {
    setBox(typed);
    const said = sizingOf(typed);
    switch (said.kind) {
      case "cleared":
        write({ ...held(), body: null });
        return;
      case "sized":
        write({ ...held(), body: said.px });
        return;
      case "refused":
        return;
    }
  };

  const faceOptions = () => cellsOf(FACES, FACE_WORDS, say);

  // A refusal the field carries, or no such property at all: an absent
  // error and an error that is nothing are different states, and only
  // the first one leaves the box unmarked.
  const refusal = (stack: string): Pick<FieldProps, "error"> =>
    stack === "" || STACK_SHAPE.test(stack) ? {} : { error: say("appearance_stack_refused") };

  const sizeRange = () =>
    say("appearance_body_range", { min: String(BODY_PX.min), max: String(BODY_PX.max) });
  const sizeRefusal = (): Pick<FieldProps, "error"> =>
    sizingOf(box()).kind === "refused" ? { error: sizeRange() } : {};

  // Chromium answers with the faces this machine has installed, after
  // asking the person. Any other engine has no such door, and the text
  // field beside it is the whole fallback.
  const offered = () => typeof window.queryLocalFonts === "function";
  const list = () => {
    const ask = window.queryLocalFonts;
    if (ask === undefined) {
      setNote(say("appearance_local_none"));
      return;
    }
    void ask()
      .then((faces) => {
        setInstalled([...new Set(faces.map((face) => face.family))].sort());
        setNote(undefined);
      })
      .catch(() => {
        setNote(say("appearance_local_denied"));
      });
  };

  // Choosing a family from this machine also pins the axis to `custom`,
  // so the choice holds however the list was reached; the axis is the
  // one the list is drawn under and is never inferred from which face
  // happens to be custom already.
  const wear = (axis: Axis, family: string) => {
    write(
      axis === "sans"
        ? { ...held(), sans: "custom", sansStack: family }
        : { ...held(), mono: "custom", monoStack: family },
    );
  };

  // The stack box and the list of installed faces for one axis, drawn
  // only while that axis is a stack the person writes. The axis's own
  // word arrives as a key rather than as a phrase, so changing the
  // language repaints the label instead of rebuilding the box a person
  // may be typing in.
  const stackFor = (axis: Axis, name: Key) => (
    <div class="flex flex-col gap-tight">
      <Field
        label={say("appearance_stack")}
        help={say("appearance_stack_help")}
        {...refusal(axis === "sans" ? held().sansStack : held().monoStack)}
        value={axis === "sans" ? held().sansStack : held().monoStack}
        mono
        onInput={(stack) => {
          write(
            axis === "sans" ? { ...held(), sansStack: stack } : { ...held(), monoStack: stack },
          );
        }}
      />
      <Show
        when={offered()}
        fallback={<p class="text-note text-text-faint">{say("appearance_local_none")}</p>}
      >
        <Button label={say("appearance_local")} onPress={list} />
      </Show>
      <Show when={note()}>
        {(said) => (
          <p class="text-note text-alert" role="alert">
            {said()}
          </p>
        )}
      </Show>
      <Show when={installed().length > 0}>
        <select
          class="w-full min-w-0 rounded-control border border-g3 bg-g2 px-base py-snug text-body text-text"
          aria-label={say("appearance_local_for", { face: say(name) })}
          onChange={(event) => {
            wear(axis, event.currentTarget.value);
          }}
        >
          <For each={installed()}>{(family) => <option value={family}>{family}</option>}</For>
        </select>
      </Show>
    </div>
  );

  return (
    <div class="flex flex-col gap-wide">
      <div class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{say("appearance_lighting")}</span>
        <Segmented
          label={say("appearance_lighting")}
          options={cellsOf(LIGHTINGS, LIGHTING_WORDS, say)}
          held={held().lighting}
          onPick={(lighting) => {
            write({ ...held(), lighting });
          }}
        />
      </div>

      <div class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{say("appearance_face")}</span>
        <Segmented
          label={say("appearance_face")}
          options={faceOptions()}
          held={held().sans}
          onPick={(sans) => {
            write({ ...held(), sans });
          }}
        />
      </div>
      <Show when={held().sans === "custom"}>{stackFor("sans", "appearance_face")}</Show>

      <div class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{say("appearance_mono")}</span>
        <Segmented
          label={say("appearance_mono")}
          options={faceOptions()}
          held={held().mono}
          onPick={(mono) => {
            write({ ...held(), mono });
          }}
        />
      </div>
      <Show when={held().mono === "custom"}>{stackFor("mono", "appearance_mono")}</Show>

      <Field
        label={say("appearance_body")}
        help={sizeRange()}
        {...sizeRefusal()}
        kind="number"
        step={1}
        suffix={say("appearance_body_unit")}
        mono
        value={box()}
        onInput={resize}
      />

      <div class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{say("appearance_preview")}</span>
        <div class="flex flex-col gap-tight rounded-card bg-g1 p-base">
          <p class="font-sans text-body text-text">{say("appearance_sample")}</p>
          <p class="font-mono text-body text-text-quiet">{say("appearance_sample")}</p>
        </div>
      </div>

      <div class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{say("appearance_density")}</span>
        <Segmented
          label={say("appearance_density")}
          options={cellsOf(DENSITIES, DENSITY_WORDS, say)}
          held={held().density}
          onPick={(density) => {
            write({ ...held(), density });
          }}
        />
      </div>

      <div class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{say("appearance_chroma")}</span>
        <Segmented
          label={say("appearance_chroma")}
          options={cellsOf(CHROMAS, CHROMA_WORDS, say)}
          held={held().chroma}
          onPick={(chroma) => {
            write({ ...held(), chroma });
          }}
        />
      </div>

      <div class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{say("appearance_motion")}</span>
        <Segmented
          label={say("appearance_motion")}
          options={cellsOf(MOTIONS, MOTION_WORDS, say)}
          held={held().motion}
          onPick={(motion) => {
            write({ ...held(), motion });
          }}
        />
      </div>
    </div>
  );
}
