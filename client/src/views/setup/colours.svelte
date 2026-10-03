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
  import { BUILT_IN_THEME, type Theme } from "../../core/prefs";
  import { ui } from "../../ui";
  import type { Editing } from "../refrain/editing";
  import { phrasesIn } from "../refrain/reading";
  import Button from "../parts/button.svelte";
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
  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "colours_tokens")}</span>
    <p class="text-note text-text-faint">{say($lang, "colours_tokens_note")}</p>
    <ul class="flex flex-col gap-tight">
      {#each tokens as token (token)}
        <li class="flex items-center gap-tight">
          <input
            type="color"
            class="h-control w-control shrink-0 rounded-control border border-edge-input bg-raised"
            aria-label={token}
            value={drawn[token] ?? ""}
            onchange={(event) => {
              pick(token, event.currentTarget.value);
            }}
          />
          <span class="min-w-0 flex-1 truncate font-mono text-note text-text-quiet">{token}</span>
          {#if token in theme.tokens}
            <button
              type="button"
              class="text-note text-text-quiet underline"
              aria-label={fill(say($lang, "colours_token_default_for"), { token })}
              onclick={() => {
                unpick(token);
              }}>{say($lang, "colours_token_default")}</button
            >
          {/if}
        </li>
      {/each}
    </ul>
  </div>
  <div class="flex flex-col gap-tight">
    <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
      <span class="text-label font-label text-text">{say($lang, "colours_legibility")}</span>
      {#if warnings.length === 0}
        <p class="text-note text-text-faint">{say($lang, "colours_fine")}</p>
      {:else}
        <ul class="flex flex-col gap-tight" role="status">
          {#each warnings as warning (warning)}
            <li class="text-note text-alert">{warning}</li>
          {/each}
        </ul>
      {/if}
    </div>
    <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
      <span class="text-label font-label text-text">{say($lang, "colours_css")}</span>
      <p class="text-note text-text-faint">{say($lang, "colours_css_note")}</p>
      <div bind:this={host} class="h-output overflow-auto rounded-control border border-edge-input bg-chrome"></div>
      <div class="flex justify-end">
        <Button label={say($lang, "colours_css_apply")} onPress={applyCss} />
      </div>
    </div>
    <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
      <span class="text-label font-label text-text">{say($lang, "colours_restore")}</span>
      <p class="text-note text-text-faint">{say($lang, "colours_restore_note")}</p>
      <div class="flex justify-end">
        <Button label={say($lang, "colours_restore")} onPress={restore} />
      </div>
    </div>
  </div>
</div>
