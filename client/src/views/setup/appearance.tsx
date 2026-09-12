// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the page is drawn: which face carries the prose, which carries a
// value read character by character, how large a line of body text is,
// how much colour the screen takes, and whether anything moves.
//
// Every choice is a name; what each name means is `theme.css`, which
// stays the one authority for a family, a size and a colour. This view
// writes an attribute on the root element and reads the result back in
// the same breath, so the page a person is looking at *is* the preview.
//
// None of it is a fact about the city, so none of it goes over the
// wire: like the language and the default effort, it lives in this
// browser's own storage.

import { For, Show, createSignal, onMount } from "solid-js";

import { useSay } from "../../ui";
import { Field } from "../parts/field";
import type { FieldProps } from "../parts/field";
import { Button } from "../parts/button";

// The Local Font Access API, which Chromium offers and other engines do
// not. Declared optional so the capability check is the type check.
declare global {
  interface Window {
    queryLocalFonts?: () => Promise<readonly { readonly family: string }[]>;
  }
}

// `system` is the absence of an opinion, and it is resolved here rather
// than in the stylesheet: the light palette is declared once, and a
// second declaration of it inside a `prefers-color-scheme` block would
// be a second authority for the same eleven rungs.
type Lighting = "system" | "dark" | "light";
type Face = "geist" | "system" | "custom";
type Body = "14" | "15" | "16";
type Chroma = "full" | "off";
// `system` is the absence of an opinion, which is what the stylesheet's
// `prefers-reduced-motion` block reads.
type Motion = "system" | "on" | "off";

export interface Appearance {
  readonly lighting: Lighting;
  readonly sans: Face;
  readonly mono: Face;
  readonly sansStack: string;
  readonly monoStack: string;
  readonly body: Body;
  readonly chroma: Chroma;
  readonly motion: Motion;
}

const LIGHTING_KEY = "sprawling.appearance.lighting";
const SANS_KEY = "sprawling.appearance.sans";
const MONO_KEY = "sprawling.appearance.mono";
const SANS_STACK_KEY = "sprawling.appearance.sans_stack";
const MONO_STACK_KEY = "sprawling.appearance.mono_stack";
const BODY_KEY = "sprawling.appearance.body";
const CHROMA_KEY = "sprawling.appearance.chroma";
const MOTION_KEY = "sprawling.appearance.motion";

const LIGHTINGS: readonly Lighting[] = ["system", "dark", "light"];
const FACES: readonly Face[] = ["geist", "system", "custom"];
const BODIES: readonly Body[] = ["14", "15", "16"];

function readFace(raw: string | null): Face {
  return FACES.find((face) => face === raw) ?? "geist";
}

function readBody(raw: string | null): Body {
  return BODIES.find((body) => body === raw) ?? "15";
}

// What a person may write into a font stack: the characters a family
// name and its punctuation are made of, and nothing that could close
// the declaration it lands in. A stack with anything else in it is not
// repaired, it is refused, and the field says so.
const STACK_SHAPE = /^[\p{L}\p{N} ,'"_-]{1,120}$/u;

function readStack(raw: string | null): string {
  return raw !== null && STACK_SHAPE.test(raw) ? raw : "";
}

export function readAppearance(store: Storage): Appearance {
  return {
    lighting: LIGHTINGS.find((lit) => lit === store.getItem(LIGHTING_KEY)) ?? "system",
    sans: readFace(store.getItem(SANS_KEY)),
    mono: readFace(store.getItem(MONO_KEY)),
    sansStack: readStack(store.getItem(SANS_STACK_KEY)),
    monoStack: readStack(store.getItem(MONO_STACK_KEY)),
    body: readBody(store.getItem(BODY_KEY)),
    chroma: store.getItem(CHROMA_KEY) === "off" ? "off" : "full",
    motion:
      store.getItem(MOTION_KEY) === "on"
        ? "on"
        : store.getItem(MOTION_KEY) === "off"
          ? "off"
          : "system",
  };
}

// One face, as the root element spells it: an attribute naming the
// choice, and - for a stack somebody typed - the stack itself, which is
// the one value the stylesheet cannot hold in advance.
function wearFace(root: HTMLElement, axis: "sans" | "mono", face: Face, stack: string): void {
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
export function watchMachineLighting(root: HTMLElement, store: Storage): void {
  window.matchMedia(MACHINE_LIGHT).addEventListener("change", () => {
    const held = readAppearance(store);
    if (held.lighting === "system") applyAppearance(root, held);
  });
}

export function applyAppearance(root: HTMLElement, held: Appearance): void {
  root.dataset.theme = held.lighting === "system" ? machineLighting() : held.lighting;
  wearFace(root, "sans", held.sans, held.sansStack);
  wearFace(root, "mono", held.mono, held.monoStack);
  root.dataset.body = held.body;
  root.dataset.chroma = held.chroma;
  root.dataset.motion = held.motion;
}

// What the page must do before it draws anything, so a person who chose
// a face last week does not meet the default face for the time it takes
// them to open this screen.
export function applyStoredAppearance(root: HTMLElement, store: Storage): void {
  applyAppearance(root, readAppearance(store));
}

// One row of exclusive choices. The options arrive already in the
// person's language; this holds no prose of its own.
function Choice<T extends string>(props: {
  readonly label: string;
  readonly options: readonly (readonly [T, string])[];
  readonly held: T;
  readonly onPick: (value: T) => void;
}) {
  return (
    <div class="flex flex-col gap-tight" role="group" aria-label={props.label}>
      <span class="text-note text-text-quiet">{props.label}</span>
      <div class="flex flex-wrap gap-tight">
        <For each={props.options}>
          {([value, label]) => (
            <button
              type="button"
              aria-pressed={props.held === value}
              class={`rounded-pill px-base py-tight text-label ${
                props.held === value ? "bg-accent text-g0" : "bg-g2 text-text-quiet hover:bg-g3"
              }`}
              onClick={() => {
                props.onPick(value);
              }}
            >
              {label}
            </button>
          )}
        </For>
      </div>
    </div>
  );
}

export function AppearanceSection() {
  const say = useSay();
  const root = document.documentElement;
  const store = localStorage;
  const [held, setHeld] = createSignal<Appearance>(readAppearance(store));
  // The faces this machine has, once a person asks for them and the
  // browser agrees. Empty until both happen.
  const [installed, setInstalled] = createSignal<readonly string[]>([]);
  const [note, setNote] = createSignal<string | undefined>(undefined);

  const write = (next: Appearance) => {
    store.setItem(LIGHTING_KEY, next.lighting);
    store.setItem(SANS_KEY, next.sans);
    store.setItem(MONO_KEY, next.mono);
    store.setItem(SANS_STACK_KEY, next.sansStack);
    store.setItem(MONO_STACK_KEY, next.monoStack);
    store.setItem(BODY_KEY, next.body);
    store.setItem(CHROMA_KEY, next.chroma);
    store.setItem(MOTION_KEY, next.motion);
    applyAppearance(root, next);
    setHeld(next);
  };

  // The screen can be reached before whatever applies this at start-up
  // has run, so it puts the stored choices on the page itself; from
  // then on `write` is what moves them.
  onMount(() => {
    applyStoredAppearance(root, store);
  });

  const faceOptions = (): readonly (readonly [Face, string])[] => [
    ["geist", say("appearance_face_geist")],
    ["system", say("appearance_face_system")],
    ["custom", say("appearance_face_custom")],
  ];

  // A refusal the field carries, or no such property at all: an absent
  // error and an error that is nothing are different states, and only
  // the first one leaves the box unmarked.
  const refusal = (stack: string): Pick<FieldProps, "error"> =>
    stack === "" || STACK_SHAPE.test(stack) ? {} : { error: say("appearance_stack_refused") };

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

  return (
    <div class="flex flex-col gap-wide">
      <Choice
        label={say("appearance_lighting")}
        options={[
          ["system", say("appearance_lighting_system")],
          ["dark", say("appearance_lighting_dark")],
          ["light", say("appearance_lighting_light")],
        ]}
        held={held().lighting}
        onPick={(lighting) => {
          write({ ...held(), lighting });
        }}
      />
      <Choice
        label={say("appearance_face")}
        options={faceOptions()}
        held={held().sans}
        onPick={(sans) => {
          write({ ...held(), sans });
        }}
      />
      <Show when={held().sans === "custom"}>
        <Field
          label={say("appearance_stack")}
          help={say("appearance_stack_help")}
          {...refusal(held().sansStack)}
          value={held().sansStack}
          mono
          onInput={(sansStack) => {
            write({ ...held(), sansStack });
          }}
        />
      </Show>

      <Choice
        label={say("appearance_mono")}
        options={faceOptions()}
        held={held().mono}
        onPick={(mono) => {
          write({ ...held(), mono });
        }}
      />
      <Show when={held().mono === "custom"}>
        <Field
          label={say("appearance_stack")}
          help={say("appearance_stack_help")}
          {...refusal(held().monoStack)}
          value={held().monoStack}
          mono
          onInput={(monoStack) => {
            write({ ...held(), monoStack });
          }}
        />
      </Show>

      <Show when={held().sans === "custom" || held().mono === "custom"}>
        <div class="flex flex-col gap-tight">
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
              aria-label={say("appearance_local")}
              onChange={(event) => {
                const family = event.currentTarget.value;
                const next = held().sans === "custom" ? { sansStack: family } : { monoStack: family };
                write({ ...held(), ...next });
              }}
            >
              <For each={installed()}>{(family) => <option value={family}>{family}</option>}</For>
            </select>
          </Show>
        </div>
      </Show>

      <Choice
        label={say("appearance_body")}
        options={BODIES.map((size) => [size, size] as const)}
        held={held().body}
        onPick={(body) => {
          write({ ...held(), body });
        }}
      />

      <div class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{say("appearance_preview")}</span>
        <div class="flex flex-col gap-tight rounded-card bg-g1 p-base">
          <p class="font-sans text-body text-text">{say("appearance_sample")}</p>
          <p class="font-mono text-body text-text-quiet">{say("appearance_sample")}</p>
        </div>
      </div>

      <Choice
        label={say("appearance_chroma")}
        options={
          [
            ["full", say("appearance_chroma_full")],
            ["off", say("appearance_chroma_none")],
          ] satisfies (readonly [Chroma, string])[]
        }
        held={held().chroma}
        onPick={(chroma) => {
          write({ ...held(), chroma });
        }}
      />

      <Choice
        label={say("appearance_motion")}
        options={
          [
            ["system", say("appearance_motion_system")],
            ["on", say("appearance_motion_full")],
            ["off", say("appearance_motion_off")],
          ] satisfies (readonly [Motion, string])[]
        }
        held={held().motion}
        onPick={(motion) => {
          write({ ...held(), motion });
        }}
      />
    </div>
  );
}
