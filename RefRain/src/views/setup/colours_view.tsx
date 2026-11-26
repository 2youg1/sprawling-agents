// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, onCleanup, onSettled, Show } from "solid-js";

import type { Appearance } from "../../core/appearance";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { BUILT_IN_THEME } from "../../core/theme_override";
import { ui } from "../../ui";
import EditorHost from "../editor/editor_host";
import type { Editing } from "../editor/editing";
import { phrasesIn } from "../editor/reading";
import Button from "../parts/button.solid";
import Glyph from "../parts/glyph.solid";
import type { Choice } from "../parts/segmented";
import { useSegmented } from "../parts/segmented.solid";
import ThemePreview from "../parts/theme_preview.look.solid";
import Card from "./card_view";
import { colourTokens, declaredNames, drawnColours, readLegibility, textClaims } from "./colours";
import type { DrawnColour, Reading } from "./colours";
import { tokenOf } from "./token";
import { TokenList } from "./token_view";

const PRESETS = [
  {
    value: "mist",
    label: "colours_preset_mist",
    appearance: { lighting: "light", neutralPalette: false },
  },
  {
    value: "ash",
    label: "colours_preset_ash",
    appearance: { lighting: "light", neutralPalette: true },
  },
  {
    value: "midnight",
    label: "colours_preset_midnight",
    appearance: { lighting: "dark", neutralPalette: false },
  },
  {
    value: "graphite",
    label: "colours_preset_graphite",
    appearance: { lighting: "dark", neutralPalette: true },
  },
] as const satisfies readonly {
  readonly value: string;
  readonly label: Key;
  readonly appearance: Pick<Appearance, "lighting" | "neutralPalette">;
}[];

type Preset = (typeof PRESETS)[number]["value"];

function drawnPreset(root: HTMLElement): Preset | null {
  const lighting = root.dataset.theme;
  const neutralPalette = root.dataset.palette === "graphite";
  return (
    PRESETS.find(
      ({ appearance }) =>
        appearance.lighting === lighting && appearance.neutralPalette === neutralPalette,
    )?.value ?? null
  );
}

function presetChoices(word: (key: Key) => string): readonly Choice<Preset>[] {
  const groups = {
    light: { label: word("colours_presets_light"), tone: "plain" },
    dark: { label: word("colours_presets_dark"), tone: "plain" },
  } as const;
  return PRESETS.map(({ value, label, appearance }) => ({
    value,
    label: word(label),
    group: groups[appearance.lighting],
  }));
}

function ThemePreviews(props: {
  readonly word: (key: Key) => string;
  readonly held: Preset | null;
  readonly pick: (value: Preset) => void;
}) {
  const look = useSegmented<Preset>({
    get label() {
      return props.word("colours_presets");
    },
    get options() {
      return presetChoices(props.word);
    },
    get held() {
      return props.held;
    },
    onPick: (value) => props.pick(value),
  });
  return (
    <div {...look().group} class="theme-previews max-w-full gap-snug">
      <For each={look().bands} keyed={(band) => band.key}>
        {(band, index) => (
          <div role="group" aria-label={band().heading} class="theme-preview-row">
            <Glyph name={index() === 0 ? "sun" : "moon"} class="text-text-quiet" />
            <For each={band().cells} keyed={(cell) => cell.key}>
              {(cell) => {
                const sample = () =>
                  PRESETS.find((preset) => preset.value === cell().key)?.appearance;
                return (
                  <ThemePreview
                    cell={cell()}
                    lighting={sample()?.lighting}
                    palette={sample()?.neutralPalette ? "graphite" : "blue"}
                  />
                );
              }}
            </For>
          </div>
        )}
      </For>
    </div>
  );
}

export default function ColoursSection() {
  const u = ui();
  const lang = storeValue(u.lang);
  const held = storeValue(u.prefs.held);
  const themeUnkept = storeValue(u.prefs.themeUnkept);
  const theme = () => held().theme;
  const word = (key: Key) => say(lang(), key);
  const root = document.documentElement;
  const [painted, setPainted] = createSignal(drawnPreset(root), { equals: false });
  const observer = new MutationObserver(() => setPainted(drawnPreset(root)));
  const [tokens, setTokens] = createSignal<readonly string[]>([]);
  const [observed, setObserved] = createSignal<{
    readonly colours: ReadonlyMap<string, DrawnColour>;
    readonly surface: string | null;
    readonly readings: readonly Reading[];
    readonly warnings: readonly string[];
  }>({ colours: new Map(), surface: null, readings: [], warnings: [] });
  let host: HTMLDivElement | undefined;
  let editing: Editing | undefined;

  function attach(next: HTMLDivElement): void {
    host = next;
    void import("../editor/editing").then(({ openEditor }) => {
      if (host !== next) return;
      editing = openEditor({
        parent: next,
        text: theme().css ?? "",
        changes: [],
        label: word("colours_css"),
        phrases: phrasesIn(lang()),
        onEdit: () => undefined,
        onSave: applyCss,
      });
    });
  }

  onSettled(() => {
    setTokens(colourTokens(declaredNames(document)));
    observer.observe(root, { attributeFilter: ["data-theme", "data-palette", "data-chroma"] });
    setPainted(drawnPreset(root));
  });

  onCleanup(() => {
    observer.disconnect();
    host = undefined;
    editing?.destroy();
    editing = undefined;
  });

  createEffect(
    () => ({ tokens: tokens(), theme: theme(), lang: lang(), painted: painted() }),
    ({ tokens, theme, lang }) => {
      const computed = window.getComputedStyle(root);
      const colours = drawnColours(root, tokens, computed);
      const judged = textClaims(tokens, (name) => computed.getPropertyValue(name));
      const warnings = [
        ...Object.keys(theme.tokens)
          .filter((token) => !tokens.includes(token))
          .map((token) => fill(say(lang, "colours_foreign"), { token })),
        ...Object.entries(theme.tokens)
          .filter(([, value]) => !CSS.supports("color", value))
          .map(([token, value]) => fill(say(lang, "colours_unreadable"), { token, value })),
      ];
      setObserved({
        colours,
        surface: judged.surface,
        readings: readLegibility(judged, (token) => colours.get(token)?.rgba ?? null),
        warnings,
      });
    },
  );

  function pickPreset(value: Preset): void {
    const next = PRESETS.find((preset) => preset.value === value);
    if (next !== undefined) u.prefs.setAppearance({ ...held().appearance, ...next.appearance });
  }

  function pick(token: string, value: string): void {
    const before = theme();
    u.prefs.setTheme({ ...before, tokens: { ...before.tokens, [token]: value } });
  }

  function unpick(token: string): void {
    const before = theme();
    u.prefs.setTheme({
      ...before,
      tokens: Object.fromEntries(Object.entries(before.tokens).filter(([name]) => name !== token)),
    });
  }

  function applyCss(): void {
    if (editing === undefined) return;
    const text = editing.text();
    u.prefs.setTheme({ ...theme(), css: text.trim() === "" ? null : text });
  }

  function restore(): void {
    u.prefs.setTheme(BUILT_IN_THEME);
    editing?.follow("");
  }

  const wells = createMemo(() =>
    tokens().map((token) => {
      const colour = observed().colours.get(token);
      return tokenOf(
        {
          name: token,
          drawn: colour?.hex,
          codes: colour?.codes ?? [],
          reset:
            token in theme().tokens
              ? {
                  word: word("colours_token_default"),
                  label: fill(word("colours_token_default_for"), { token }),
                }
              : undefined,
        },
        { pick, unpick },
      );
    }),
  );

  return (
    <div class="grid grid-fit items-start gap-base">
      <div class="colours-presets">
        <Card title={word("colours_presets")}>
          <ThemePreviews word={word} held={painted()} pick={pickPreset} />
        </Card>
      </div>
      <Card title={word("colours_tokens")} note={word("colours_tokens_note")}>
        <TokenList tokens={wells()} />
      </Card>
      <div class="flex flex-col gap-base">
        <Card title={word("colours_legibility")}>
          <Show when={observed().surface}>
            {(name) => (
              <p class="text-note text-text-faint">
                {fill(word("colours_apca_note"), { surface: name() })}
              </p>
            )}
          </Show>
          <ul class="flex flex-col gap-snug">
            <For each={observed().readings} keyed={(reading) => reading.token}>
              {(reading) => (
                <li class="grid gap-hair text-note">
                  <code class="wrap-anywhere">{reading().token}</code>
                  <span class={["tabular-nums", { "text-alert": !meetsTier(reading()) }]}>
                    {contrastLabel(reading(), word)}
                  </span>
                </li>
              )}
            </For>
          </ul>
          <Show when={observed().warnings.length > 0}>
            <ul role="status" class="flex flex-col gap-tight">
              <For each={observed().warnings}>
                {(warning) => <li class="text-note text-alert">{warning}</li>}
              </For>
            </ul>
          </Show>
        </Card>
        <Show when={themeUnkept()}>
          <p role="status" class="text-note text-alert">
            {word("colours_only_tab")}
          </p>
        </Show>
        <Card title={word("colours_css")} note={word("colours_css_note")}>
          <div class="flex h-output flex-col overflow-hidden rounded-control border border-edge-input">
            <EditorHost ref={attach} />
          </div>
          <div class="flex justify-end">
            <Button label={word("colours_css_apply")} onPress={applyCss} />
          </div>
        </Card>
        <Card title={word("colours_restore")} note={word("colours_restore_note")}>
          <div class="flex justify-end">
            <Button label={word("colours_restore")} onPress={restore} />
          </div>
        </Card>
      </div>
    </div>
  );
}

function meetsTier(reading: Reading): boolean {
  return reading.kind === "measured" && reading.meetsTier;
}

function contrastLabel(reading: Reading, word: (key: Key) => string): string {
  const target = String(reading.tier);
  if (reading.kind === "unmeasured") return fill(word("colours_apca_unmeasured"), { target });
  return fill(word(reading.meetsTier ? "colours_apca_value" : "colours_apca_short"), {
    value: reading.reached.toFixed(1),
    target,
  });
}
