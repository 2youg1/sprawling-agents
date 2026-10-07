<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The local looks of the setup groups, each in the states its seat
  // hands it: a card whose change just landed beside one at rest, the
  // blend slider stated and unstated, a shortcut row listening and idle,
  // colour tokens with and without an override, a door off the advanced
  // group, the governed document's box, and the folded `CONFIG.toml`.
  // The words are the ones the seats translate, so the fixture follows
  // the page's language.
  import { BLEND_PERCENT } from "../../core/appearance";
  import { chordOf } from "../setup/chord";
  import { draftOf } from "../setup/draft";
  import { doorOf } from "../setup/door";
  import { sliderOf } from "../setup/slider";
  import { tokenOf } from "../setup/token";

  const CARD = 360;
  const percent = (at: number): string => `${String(at)}%`;
  const still = (): void => undefined;
  const HANDS = { listen: still, bind: still };
  const TOKENS = { pick: still, unpick: still };
  // wording-ok: fixture text is a file the city holds, not a page's words
  const TOML = ["[ui]", 'lang = "en"', "body_px = 15"].join("\n");
  // wording-ok: fixture text is a document the person wrote, not a page's words
  const MAYOR = ["# Mayor", "", "Answer in the language the person wrote in."].join("\n");
</script>

<script lang="ts">
  import { LABELS } from "../../core/keys";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import EmptyState from "../parts/empty.svelte";
  import { receiptOf } from "../setup/card";
  import Card from "../setup/card.look.svelte";
  import ChordLook from "../setup/chord.look.svelte";
  import Door from "../setup/door.look.svelte";
  import Draft from "../setup/draft.look.svelte";
  import Slider from "../setup/slider.look.svelte";
  import Token from "../setup/token.look.svelte";
  import TomlLook from "../setup/toml.look.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
  const range = $derived(
    fill(say($lang, "appearance_blend_range"), {
      min: String(BLEND_PERCENT.min),
      max: String(BLEND_PERCENT.max),
      step: String(BLEND_PERCENT.step),
    }),
  );
  const chord = $derived({ spelled: "accel+k", prompt: say($lang, "keys_press"), tip: say($lang, "keys_change") });
  const reset = (token: string): { readonly word: string; readonly label: string } => ({
    word: say($lang, "colours_token_default"),
    label: fill(say($lang, "colours_token_default_for"), { token }),
  });
</script>

{#snippet slider(at: number | null)}
  <Slider {...sliderOf({ label: say($lang, "appearance_blend"), range: BLEND_PERCENT, at, figure: percent, onMove: still })} />
{/snippet}

<Case label="setup card · a change just landed, with its range" width={CARD}>
  <Card
    title={say($lang, "appearance_blend")}
    note={say($lang, "appearance_blend_note")}
    constraint={range}
    receipt={receiptOf(true, say($lang, "setup_saved"))}
  >
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render slider(60)}
  </Card>
</Case>

<Case label="setup card · at rest, nothing stated" width={CARD}>
  <Card
    title={say($lang, "appearance_blend")}
    note={say($lang, "appearance_blend_note")}
    constraint={range}
    receipt={receiptOf(false, say($lang, "setup_saved"))}
  >
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render slider(null)}
  </Card>
</Case>

<Case label="setup card · a door off the advanced group" width={CARD}>
  <Card title={say($lang, "privacy_open")} note={say($lang, "privacy_open_note")}>
    <Door {...doorOf({ kind: "setup", group: "privacy" }, say($lang, "setup_group_privacy"))} />
  </Card>
</Case>

<Case label="setup keys · one row listening, one at rest" width={CARD}>
  <ul class="flex flex-col">
    <li class="flex items-center gap-base border-b border-edge py-snug text-label">
      <span class="min-w-0 flex-1 truncate text-text-quiet">{say($lang, LABELS.palette)}</span>
      <ChordLook {...chordOf({ ...chord, action: "palette", listening: true }, HANDS)} />
    </li>
    <li class="flex items-center gap-base py-snug text-label">
      <span class="min-w-0 flex-1 truncate text-text-quiet">{say($lang, LABELS.finder)}</span>
      <ChordLook {...chordOf({ ...chord, action: "finder", listening: false }, HANDS)} />
    </li>
  </ul>
</Case>

<Case label="setup colours · a token overridden and one as drawn" width={CARD}>
  <ul class="flex flex-col gap-tight">
    <Token {...tokenOf({ name: "--color-accent", drawn: undefined, reset: reset("--color-accent") }, TOKENS)} />
    <Token {...tokenOf({ name: "--color-edge", drawn: undefined, reset: undefined }, TOKENS)} />
  </ul>
</Case>

<Case label="setup governed · the document box" width={CARD}>
  <Draft {...draftOf(say($lang, "governed_mayor"), MAYOR, still)} />
</Case>

<Case label="setup toml · the file as the city holds it">
  <TomlLook
    region={{ "aria-label": say($lang, "setup_toml") }}
    toggle={say($lang, "setup_toml_toggle")}
    path=".sprawling/CONFIG.toml"
    text={TOML}
  >
    {#snippet unread()}
      <EmptyState missing="setup_toml_unread" />
    {/snippet}
  </TomlLook>
</Case>
