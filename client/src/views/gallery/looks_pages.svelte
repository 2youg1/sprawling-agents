<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The looks of RefRain, the settings panel and the monitor's code
  // column, each drawn from a value of its own type with the states its
  // seat hands it, so the roles and boxes each look draws are read on
  // this page rather than only inside a whole screen.
  import { say } from "../../core/lang";
  import type { View } from "../../core/route";
  import { ui } from "../../ui";
  import CodeColumn from "../monitor/code_column.look.svelte";
  import Conflict from "../refrain/conflict.look.svelte";
  import Editor from "../refrain/editor.look.svelte";
  import { editorWire } from "../refrain/editor";
  import Line from "../refrain/line.look.svelte";
  import Pair from "../refrain/pair.look.svelte";
  import { sideWire } from "../refrain/pair";
  import Card from "../settings/card.look.svelte";
  import DoorKey from "../settings/door_key.look.svelte";
  import { foldsOf } from "../settings/folds";
  import Panel from "../settings/panel.look.svelte";
  import { dialogOf } from "../settings/panel";
  import Tree from "../settings/tree.look.svelte";
  import { lookOf as treeLookOf } from "../settings/tree";
  import Case from "./case.svelte";

  const { lang } = ui();
  const still = (): void => undefined;
  // Fixture data: what a city or a person would have written.
  const WORDS = { compare: "compare", held: "The words this card holds." };
  const kept = (): (() => void) => still;
  const BENEATH: View = { kind: "city" };
</script>

{#snippet compare()}
  <span class="text-note">{WORDS.compare}</span>
{/snippet}

{#snippet held()}
  <p class="text-note text-text-faint">{WORDS.held}</p>
{/snippet}

{#snippet nothing()}{/snippet}

<Case label="refrain looks · a conflict bar, two lines, a version pair and the editor's box">
  <div class="flex flex-col gap-base">
    <Conflict text="Somebody else saved Plan.md while you were editing it." wire={{ role: "alert" }} actions={compare} />
    <Line tone="faint" children={held} />
    <Line tone="alert" children={held} />
    <Pair
      sides={[
        {
          key: "left",
          label: "before",
          held: "v2",
          options: [
            { value: "v1", label: "v1", disabled: false },
            { value: "v2", label: "v2", disabled: false },
          ],
          wire: sideWire("before", still),
        },
        {
          key: "right",
          label: "after",
          held: "v3",
          options: [
            { value: "v2", label: "v2", disabled: true },
            { value: "v3", label: "v3", disabled: false },
          ],
          wire: sideWire("after", still),
        },
      ]}
      more={null}
    />
    <div class="flex h-output flex-col overflow-hidden rounded-control border border-edge-input">
      <Editor wire={editorWire(still)} />
    </div>
  </div>
</Case>

<Case label="settings looks · the tree on the appearance group, a card saving, and a door key">
  <div class="flex items-start gap-base">
    <div class="w-index">
      <Tree
        {...treeLookOf(
          {
            group: "appearance",
            beneath: BENEATH,
            folds: foldsOf("appearance", BENEATH),
            leaves: { buildings: [], record: [] },
            reading: null,
          },
          "fixture-tree",
          { words: (key) => say($lang, key), pick: still, toggle: still, walk: still, hold: kept },
        )}
      />
    </div>
    <div class="flex min-w-0 flex-1 flex-col gap-base">
      <Card
        title={say($lang, "colours_tokens")}
        note={say($lang, "colours_tokens_note")}
        standing={{ word: say($lang, "saving_sent"), weight: "quiet", saved: false, detail: null }}
        press={{ label: say($lang, "colours_css_apply"), loading: true, why: undefined, onPress: still }}
        children={held}
      />
      <DoorKey glyph="check" label="open the door" note="for 12 hours" wire={{ type: "button", "aria-disabled": false, onclick: still }} />
    </div>
  </div>
  <Panel dialog={dialogOf("fixture-panel-title", still, kept)} children={nothing} />
</Case>

<Case label="monitor code column · one file, one hunk with its presses, and the empty column">
  <div class="flex flex-col gap-base">
    <CodeColumn
      empty={undefined}
      files={[
        {
          path: "shop/checkout/total.rs",
          how: "M",
          hunks: [
            {
              actions: [
                { key: "comment", label: "comment", kind: "press", wire: { type: "button", onclick: still } },
                { key: "open", label: "open", kind: "open", wire: { href: "#/building/shop" } },
              ],
              lines: [
                { sign: "kept", old: "11", new: "11", text: "let total = items.sum();" },
                { sign: "removed", old: "12", new: "", text: "total" },
                { sign: "added", old: "", new: "12", text: "total + tax" },
              ],
            },
          ],
        },
      ]}
    />
    <CodeColumn empty="Nothing changed yet." files={[]} />
  </div>
</Case>
