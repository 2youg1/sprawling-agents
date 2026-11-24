// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, onCleanup, onSettled, Show } from "solid-js";

import type { Rgba } from "../../core/apca";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { BUILT_IN_THEME } from "../../core/theme_override";
import { ui } from "../../ui";
import EditorHost from "../editor/editor_host";
import type { Editing } from "../editor/editing";
import { phrasesIn } from "../editor/reading";
import Button from "../parts/button.solid";
import Card from "./card_view";
import { colourTokens, declaredNames, drawnRgb, shortfalls, textClaims } from "./colours";
import { tokenOf } from "./token";
import { TokenList } from "./token_view";

function hexOf([red, green, blue]: Rgba): string {
  return `#${[red, green, blue].map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
}

export default function ColoursSection() {
  const u = ui();
  const lang = storeValue(u.lang);
  const held = storeValue(u.prefs.held);
  const themeUnkept = storeValue(u.prefs.themeUnkept);
  const theme = () => held().theme;
  const word = (key: Key) => say(lang(), key);
  const [tokens, setTokens] = createSignal<readonly string[]>([]);
  const [drawn, setDrawn] = createSignal<Readonly<Record<string, string>>>({});
  const [warnings, setWarnings] = createSignal<readonly string[]>([]);
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
  });

  onCleanup(() => {
    host = undefined;
    editing?.destroy();
    editing = undefined;
  });

  createEffect(
    () => ({ tokens: tokens(), theme: theme(), lang: lang() }),
    ({ tokens, theme, lang }) => {
      const resolve = drawnRgb(document.documentElement);
      setDrawn(
        Object.fromEntries(
          tokens.flatMap((token) => {
            const rgb = resolve(token);
            return rgb === null ? [] : [[token, hexOf(rgb)]];
          }),
        ),
      );
      const computed = window.getComputedStyle(document.documentElement);
      const judged = textClaims(tokens, (name) => computed.getPropertyValue(name));
      setWarnings([
        ...Object.keys(theme.tokens)
          .filter((token) => !tokens.includes(token))
          .map((token) => fill(say(lang, "colours_foreign"), { token })),
        ...Object.entries(theme.tokens)
          .filter(([, value]) => !CSS.supports("color", value))
          .map(([token, value]) => fill(say(lang, "colours_unreadable"), { token, value })),
        ...shortfalls(judged, resolve).map((short) =>
          short.kind === "unmeasured"
            ? fill(say(lang, "colours_unmeasured"), { token: short.token })
            : fill(say(lang, "colours_short"), {
                token: short.token,
                reached: short.reached.toFixed(1),
                claimed: String(short.claimed),
              }),
        ),
      ]);
    },
  );

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
    tokens().map((token) =>
      tokenOf(
        {
          name: token,
          drawn: drawn()[token],
          reset:
            token in theme().tokens
              ? {
                  word: word("colours_token_default"),
                  label: fill(word("colours_token_default_for"), { token }),
                }
              : undefined,
        },
        { pick, unpick },
      ),
    ),
  );

  return (
    <div class="grid grid-fit items-start gap-base">
      <Card title={word("colours_tokens")} note={word("colours_tokens_note")}>
        <TokenList tokens={wells()} />
      </Card>
      <div class="flex flex-col gap-base">
        <Card title={word("colours_legibility")}>
          <div role="status">
            <Show
              when={warnings().length > 0}
              fallback={<p class="text-note text-text-faint">{word("colours_fine")}</p>}
            >
              <ul class="flex flex-col gap-tight">
                <For each={warnings()}>
                  {(warning) => <li class="text-note text-alert">{warning}</li>}
                </For>
              </ul>
            </Show>
          </div>
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
