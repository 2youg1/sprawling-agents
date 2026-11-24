// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, For, Show } from "solid-js";

import {
  FACES,
  FAMILY_SHAPE,
  FEATURES,
  READING_FACES,
  WEIGHTS,
  featuresOffered,
} from "../../core/appearance";
import type { Appearance, TypeFeature } from "../../core/appearance";
import { mintIdem } from "../../core/idem";
import { fill, say, type Key } from "../../core/lang";
import { LINE_HEIGHT_FLOOR, TRACKING_FLOOR, ratioOf, spelledRatio } from "../../core/metrics";
import { sizingOf } from "../../core/sizing";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { BODY_PX, type AxError } from "../../wire";
import { DiagnosticView } from "../diagnostic_view";
import Button from "../parts/button.solid";
import type { Choice } from "../parts/combobox";
import Combobox from "../parts/combobox_view";
import Field from "../parts/field.solid";
import Segmented from "../parts/segmented.solid";
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
  type Axis,
  type Setting,
} from "./appearance";
import type { ReceiptLook } from "./card";
import Card from "./card_view";

const SWITCHES = ["off", "on"] as const;
const SWITCH_WORDS = {
  off: "appearance_feature_off",
  on: "appearance_feature_on",
} satisfies Record<(typeof SWITCHES)[number], Key>;

interface AppearanceTypeProps {
  readonly look: Appearance;
  readonly write: (next: Appearance, name: Setting) => void;
  readonly receipt: (name: Setting) => ReceiptLook;
}

export default function AppearanceType(props: AppearanceTypeProps) {
  const lang = storeValue(ui().lang);
  const word = (key: Key) => say(lang(), key);
  const look = () => props.look;
  const root = document.documentElement;
  const initial = look();
  const [body, setBody] = createSignal(
    initial.body === null ? drawnSize(root) : String(initial.body),
  );
  const [bodyIssue, setBodyIssue] = createSignal<AxError>();
  const [fallbacks, setFallbacks] = createSignal<readonly string[]>(initial.fallbacks);
  const [leading, setLeading] = createSignal(
    initial.lineHeight === null ? drawnLeading(root) : spelledRatio(initial.lineHeight),
  );
  const [tracking, setTracking] = createSignal(
    initial.tracking === null ? "" : spelledRatio(initial.tracking),
  );
  const [installed, setInstalled] = createSignal<readonly string[]>([]);
  const [fontNote, setFontNote] = createSignal<string>();
  const localChoices = createMemo<readonly Choice[]>(() =>
    installed().map((family) => ({ value: family, label: family, face: faceStyle(family) })),
  );
  const live = createMemo(() => featuresOffered([look().sans, look().mono]));

  function resize(typed: string): void {
    setBody(typed);
    const parsed = sizingOf(typed);
    if (parsed.kind === "refused") {
      const known = typed.trim();
      setBodyIssue({
        code: "E_INVALID_ARGS",
        action: word("appearance_body"),
        subject: "Appearance.body_px",
        request: mintIdem(),
        nearby: [
          'stage = "validate"',
          `minimum = ${String(BODY_PX.min)}`,
          ...(/^-?[0-9]+(?:\.[0-9]+)?$/.test(known) ? [`actual = ${JSON.stringify(known)}`] : []),
          'outcome = "unsent"',
        ],
        recovery: fill(word("appearance_body_floor"), { min: String(BODY_PX.min) }),
        retry: "no",
      });
      return;
    }
    setBodyIssue(undefined);
    props.write({ ...look(), body: parsed.kind === "cleared" ? null : parsed.px }, "body");
  }

  function metric(name: "lineHeight" | "tracking", typed: string): void {
    const leadingMetric = name === "lineHeight";
    if (leadingMetric) setLeading(typed);
    else setTracking(typed);
    const parsed = ratioOf(typed, leadingMetric ? LINE_HEIGHT_FLOOR : TRACKING_FLOOR);
    if (parsed.kind !== "refused") {
      props.write(
        { ...look(), [name]: parsed.kind === "cleared" ? null : parsed.perMille },
        "metrics",
      );
    }
  }

  function fallback(slot: 0 | 1, typed: string): void {
    const next = [
      slot === 0 ? typed : (fallbacks()[0] ?? ""),
      slot === 1 ? typed : (fallbacks()[1] ?? ""),
    ];
    setFallbacks(next);
    if (!next.every((name) => name === "" || FAMILY_SHAPE.test(name))) return;
    const valid = next.filter((name) => name !== "");
    props.write({ ...look(), fallbacks: valid }, "fallbacks");
    setFallbacks(valid);
  }

  function feature(name: TypeFeature, enabled: boolean): void {
    const features = FEATURES.filter((item) =>
      item === name ? enabled : look().features.includes(item),
    );
    props.write({ ...look(), features }, "features");
  }

  const canList = typeof window.queryLocalFonts === "function";
  function list(): void {
    if (window.queryLocalFonts === undefined) return;
    void window
      .queryLocalFonts()
      .then((faces) => {
        setInstalled(
          [
            ...new Set(
              faces.map((face) => face.family).filter((family) => FAMILY_SHAPE.test(family)),
            ),
          ].sort((a, b) => a.localeCompare(b)),
        );
        setFontNote(undefined);
      })
      .catch(() => setFontNote(word("appearance_local_denied")));
  }

  function LocalAsk() {
    return (
      <>
        <Show
          when={canList}
          fallback={<p class="text-note text-text-faint">{word("appearance_local_none")}</p>}
        >
          <Show when={installed().length === 0}>
            <Button label={word("appearance_local")} glyph="search" onPress={list} />
          </Show>
        </Show>
        <Show when={fontNote()}>
          <p class="text-note text-alert" role="alert">
            {fontNote()}
          </p>
        </Show>
      </>
    );
  }

  function LocalPicker(local: {
    readonly label: string;
    readonly held: string;
    readonly onPick: (family: string) => void;
  }) {
    return (
      <Show when={localChoices().length > 0}>
        <Combobox
          label={local.label}
          placeholder={word("part_search")}
          empty={word("part_no_match")}
          choices={localChoices()}
          value={local.held}
          onPick={local.onPick}
        />
      </Show>
    );
  }

  function Typeface(part: { readonly axis: Axis; readonly label: Key; readonly note: Key }) {
    const field = part.axis === "sans" ? "sansStack" : "monoStack";
    const name = part.axis === "sans" ? "face" : "mono";
    const stack = () => look()[field];
    function writeStack(next: string): void {
      props.write({ ...look(), [field]: next }, name);
    }
    return (
      <Card title={word(part.label)} note={word(part.note)} receipt={props.receipt(name)}>
        <Segmented
          label={word(part.label)}
          options={cellsOf(FACES, FACE_WORDS, word)}
          held={look()[part.axis]}
          onPick={(value) => props.write({ ...look(), [part.axis]: value }, name)}
        />
        <Show when={look()[part.axis] === "custom"}>
          <div class="flex flex-col gap-tight">
            <Field
              label={word("appearance_stack")}
              help={word("appearance_stack_help")}
              {...(stackRefused(stack()) ? { error: word("appearance_stack_refused") } : {})}
              value={stack()}
              mono
              onInput={writeStack}
            />
            <LocalAsk />
            <LocalPicker
              label={fill(word("appearance_local_for"), { face: word(part.label) })}
              held={stack()}
              onPick={writeStack}
            />
          </div>
        </Show>
      </Card>
    );
  }

  function Fallback(part: { readonly slot: 0 | 1; readonly label: Key }) {
    const typed = () => fallbacks()[part.slot] ?? "";
    const disabled = () => part.slot === 1 && (fallbacks()[0] ?? "").trim() === "";
    const help = () =>
      part.slot === 0
        ? {}
        : {
            help: word(
              disabled() ? "appearance_fallback_needs_first" : "appearance_fallback_second_help",
            ),
          };
    const error = () =>
      typed() !== "" && !FAMILY_SHAPE.test(typed())
        ? { error: word("appearance_fallback_refused") }
        : {};
    const onInput = (next: string): void => fallback(part.slot, next);
    return (
      <>
        <Field
          label={word(part.label)}
          {...help()}
          {...error()}
          value={typed()}
          disabled={disabled()}
          onInput={onInput}
        />
        <Show when={!disabled()}>
          <LocalPicker label={word(part.label)} held={typed()} onPick={onInput} />
        </Show>
      </>
    );
  }

  const sizeFloor = () => fill(word("appearance_body_floor"), { min: String(BODY_PX.min) });
  const leadingFloor = () =>
    fill(word("appearance_leading_floor"), { min: spelledRatio(LINE_HEIGHT_FLOOR) });
  const trackingFloor = () =>
    fill(word("appearance_tracking_floor"), { min: spelledRatio(TRACKING_FLOOR) });
  const bodyError = () =>
    bodyIssue() === undefined
      ? {}
      : {
          error: fill(word("appearance_body_not_saved"), {
            actual: body(),
            min: String(BODY_PX.min),
          }),
        };
  const leadingError = () =>
    ratioOf(leading(), LINE_HEIGHT_FLOOR).kind === "refused" ? { error: leadingFloor() } : {};
  const trackingError = () =>
    ratioOf(tracking(), TRACKING_FLOOR).kind === "refused" ? { error: trackingFloor() } : {};
  return (
    <>
      <Typeface axis="sans" label="appearance_face" note="appearance_face_note" />
      <Typeface axis="mono" label="appearance_mono" note="appearance_mono_note" />
      <Card
        title={word("appearance_reading")}
        note={word("appearance_reading_note")}
        receipt={props.receipt("reading")}
      >
        <Segmented
          label={word("appearance_reading")}
          options={cellsOf(READING_FACES, READING_WORDS, word)}
          held={look().reading}
          onPick={(reading) => props.write({ ...look(), reading }, "reading")}
        />
      </Card>
      <Card
        title={word("appearance_fallbacks")}
        note={word("appearance_fallbacks_note")}
        constraint={word("appearance_fallbacks_help")}
        receipt={props.receipt("fallbacks")}
      >
        <div class="flex flex-col gap-tight">
          <LocalAsk />
          <Fallback slot={0} label="appearance_fallback_first" />
          <Fallback slot={1} label="appearance_fallback_second" />
        </div>
      </Card>
      <Card
        title={word("appearance_body")}
        note={word("appearance_body_note")}
        constraint={sizeFloor()}
        receipt={props.receipt("body")}
      >
        <Field
          label={word("appearance_body")}
          labelling="hidden"
          {...bodyError()}
          kind="number"
          step={1}
          suffix={word("appearance_body_unit")}
          mono
          value={body()}
          onInput={resize}
        />
        <Show when={bodyIssue()}>
          {(error) => <DiagnosticView error={error()} lang={lang()} />}
        </Show>
      </Card>
      <Card
        title={word("appearance_metrics")}
        note={word("appearance_metrics_note")}
        receipt={props.receipt("metrics")}
      >
        <div class="flex flex-col gap-base">
          <div class="flex flex-col gap-tight">
            <Field
              label={word("appearance_leading")}
              {...leadingError()}
              value={leading()}
              onInput={(typed) => metric("lineHeight", typed)}
            />
            <p class="text-note text-text-faint">{leadingFloor()}</p>
          </div>
          <div class="flex flex-col gap-tight">
            <Field
              label={word("appearance_tracking")}
              {...trackingError()}
              suffix={word("appearance_tracking_unit")}
              value={tracking()}
              onInput={(typed) => metric("tracking", typed)}
            />
            <p class="text-note text-text-faint">{trackingFloor()}</p>
          </div>
          <div class="flex min-w-0 flex-col gap-snug">
            <span class="text-note text-text-quiet">{word("appearance_weight")}</span>
            <Segmented
              label={word("appearance_weight")}
              options={cellsOf(WEIGHTS, WEIGHT_WORDS, word)}
              held={look().weight}
              onPick={(weight) => props.write({ ...look(), weight }, "metrics")}
            />
            <p class="text-note text-text-faint">{word("appearance_weight_note")}</p>
          </div>
        </div>
      </Card>
      <Card
        title={word("appearance_features")}
        note={word("appearance_features_note")}
        receipt={props.receipt("features")}
      >
        <div class="flex flex-col gap-snug">
          <For each={FEATURES} keyed={true}>
            {(name) => {
              const available = () => live().has(name);
              const options = () =>
                cellsOf(SWITCHES, SWITCH_WORDS, word).map((cell) =>
                  available() ? cell : { ...cell, why: word("appearance_feature_absent") },
                );
              return (
                <div class="flex flex-wrap items-center justify-between gap-base">
                  <span class="text-note text-text-quiet">{word(FEATURE_WORDS[name])}</span>
                  <Segmented
                    label={word(FEATURE_WORDS[name])}
                    options={options()}
                    held={available() && look().features.includes(name) ? "on" : "off"}
                    onPick={(next) => feature(name, next === "on")}
                  />
                </div>
              );
            }}
          </For>
        </div>
      </Card>
      <Card title={word("appearance_preview")} note={word("appearance_preview_note")}>
        <div class="flex flex-col gap-base rounded-card bg-chrome p-base">
          <div class="flex flex-col gap-tight">
            <span class="text-note text-text-faint">{word("appearance_face")}</span>
            <p class="font-sans text-body text-text">{word("appearance_sample")}</p>
          </div>
          <div class="flex flex-col gap-tight">
            <span class="text-note text-text-faint">{word("appearance_mono")}</span>
            <p class="font-mono text-body text-text-quiet">{word("appearance_sample")}</p>
          </div>
          <div class="flex flex-col gap-tight">
            <span class="text-note text-text-faint">{word("appearance_reading")}</span>
            <p class="font-read text-body leading-read text-text" data-read={look().reading}>
              {word("appearance_reading_sample")}
            </p>
          </div>
        </div>
      </Card>
    </>
  );
}
