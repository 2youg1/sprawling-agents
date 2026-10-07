<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The colour page (roadmap CT): every colour token the theme draws
  // with, a picker for each, a stylesheet of the person's own in
  // RefRain's editor, what the legibility rule says about the result,
  // and the way back to the built-in theme.
  //
  // The record is `core/prefs.ts`'s and the wearing is `main.ts`'s, so
  // this file only names changes through the door; the page a person is
  // looking at is the preview. The tokens and the tiers are read off the
  // running page by `colours.ts`, never listed here.

  import { onMount } from "svelte";

  import { fill, say } from "../../core/lang";
  import { BUILT_IN_THEME, type Theme } from "../../core/theme_override";
  import { ui } from "../../ui";
  import { editorWire } from "../refrain/editor";
  import Editor from "../refrain/editor.look.svelte";
  import type { Editing } from "../refrain/editing";
  import { phrasesIn } from "../refrain/reading";
  import Button from "../parts/button.svelte";
  import Card from "./card.look.svelte";
  import { tokenOf } from "./token";
  import Token from "./token.look.svelte";
  import { colourTokens, declaredNames, drawnRgb, shortfalls, textClaims, type Rgb } from "./colours";

  const u = ui();
  const lang = u.lang;
  const held = u.prefs.held;
  const theme: Theme = $derived($held.theme);

  // The tokens the running page declares, read once it is mounted.
  let tokens = $state<readonly string[]>([]);
  // The colour each token draws now, as `#rrggbb` for the picker; read
  // again after every change, since one token may be a hop to another.
  let drawn = $state<Readonly<Record<string, string>>>({});
  let warnings = $state<readonly string[]>([]);
  let host = $state<HTMLDivElement | undefined>(undefined);
  const wire = editorWire((box) => {
    host = box;
  });
  let editing: Editing | undefined;

  onMount(() => {
    tokens = colourTokens(declaredNames(document));
    let gone = false;
    void import("../refrain/editing").then(({ openEditor }) => {
      if (gone || host === undefined) return;
      editing = openEditor({
        parent: host,
        text: theme.css ?? "",
        changes: [],
        label: say($lang, "colours_css"),
        phrases: phrasesIn($lang),
        onEdit: () => undefined,
        onSave: applyCss,
      });
    });
    return () => {
      gone = true;
      editing?.destroy();
    };
  });

  // Every change to the theme is worn before this runs (the shell
  // subscribes first), so the computed style already answers for it.
  $effect(() => {
    const resolve = drawnRgb(document.documentElement);
    drawn = Object.fromEntries(tokens.flatMap((token) => {
      const rgb = resolve(token);
      return rgb === null ? [] : [[token, hexOf(rgb)]];
    }));
    const computed = window.getComputedStyle(document.documentElement);
    const judged = textClaims(tokens, (name) => computed.getPropertyValue(name));
    warnings = [
      ...Object.keys(theme.tokens)
        .filter((token) => !tokens.includes(token))
        .map((token) => fill(say($lang, "colours_foreign"), { token })),
      ...Object.entries(theme.tokens)
        .filter(([, value]) => !CSS.supports("color", value))
        .map(([token, value]) => fill(say($lang, "colours_unreadable"), { token, value })),
      ...shortfalls(judged, resolve).map((short) =>
        fill(say($lang, "colours_short"), {
          token: short.token,
          reached: short.reached.toFixed(1),
          claimed: String(short.claimed),
        }),
      ),
    ];
  });

  function hexOf(rgb: Rgb): string {
    return `#${rgb.map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
  }

  function pick(token: string, value: string): void {
    u.prefs.setTheme({ ...theme, tokens: { ...theme.tokens, [token]: value } });
  }

  function unpick(token: string): void {
    u.prefs.setTheme({ ...theme, tokens: Object.fromEntries(Object.entries(theme.tokens).filter(([name]) => name !== token)) });
  }

  function applyCss(): void {
    const text = editing?.text() ?? "";
    u.prefs.setTheme({ ...theme, css: text.trim() === "" ? null : text });
  }

  function restore(): void {
    u.prefs.setTheme(BUILT_IN_THEME);
    editing?.follow("");
  }
</script>

<div class="grid grid-fit items-start gap-base">
  <Card title={say($lang, "colours_tokens")} note={say($lang, "colours_tokens_note")}>
    <ul class="flex flex-col gap-tight">
      {#each tokens as token (token)}
        <Token
          {...tokenOf(
            {
              name: token,
              drawn: drawn[token],
              reset:
                token in theme.tokens
                  ? {
                      word: say($lang, "colours_token_default"),
                      label: fill(say($lang, "colours_token_default_for"), { token }),
                    }
                  : undefined,
            },
            { pick, unpick },
          )}
        />
      {/each}
    </ul>
  </Card>
  <div class="flex flex-col gap-base">
    <Card title={say($lang, "colours_legibility")}>
      <!-- The region stands while all is well too, so a warning that
      arrives is announced: a live region is only heard when it changes
      after it was put on the page. -->
      <div role="status">
        {#if warnings.length === 0}
          <p class="text-note text-text-faint">{say($lang, "colours_fine")}</p>
        {:else}
          <ul class="flex flex-col gap-tight">
            {#each warnings as warning (warning)}
              <li class="text-note text-alert">{warning}</li>
            {/each}
          </ul>
        {/if}
      </div>
    </Card>
    <Card title={say($lang, "colours_css")} note={say($lang, "colours_css_note")}>
      <!-- The editor is RefRain's, so it is mounted in RefRain's box and
      takes its dress: outside it CodeMirror falls back to its own light
      theme and paints the active line's gutter as a light block on a
      dark page. -->
      <div class="flex h-output flex-col overflow-hidden rounded-control border border-edge-input">
        <Editor {wire} />
      </div>
      <div class="flex justify-end">
        <Button label={say($lang, "colours_css_apply")} onPress={applyCss} />
      </div>
    </Card>
    <Card title={say($lang, "colours_restore")} note={say($lang, "colours_restore_note")}>
      <div class="flex justify-end">
        <Button label={say($lang, "colours_restore")} onPress={restore} />
      </div>
    </Card>
  </div>
</div>
