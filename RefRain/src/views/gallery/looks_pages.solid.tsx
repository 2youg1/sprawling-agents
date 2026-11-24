// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import type { View } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Conflict from "../editor/conflict.look.solid";
import EditorHost from "../editor/editor_host";
import Line from "../editor/line.look.solid";
import Pair from "../editor/pair.look.solid";
import { sideWire } from "../editor/pair";
import type { CodeColumnLook } from "../monitor/code_column";
import { CodeColumnView } from "../monitor/trace_view";
import { foldsOf } from "../settings/folds";
import { PanelDialog } from "../settings/panel_view";
import { DoorKey } from "../settings/remote_door_view";
import SaveCard from "../settings/save_card_view";
import { sent } from "../settings/saving";
import TreeControls from "../settings/tree_controls";
import { lookOf as treeLookOf } from "../settings/tree";
import { GalleryCase } from "./case";

const still = (): void => undefined;
const BENEATH: View = { kind: "city" };
const WORDS = { compare: "compare", held: "The words this card holds." };
const DIFF: CodeColumnLook = {
  empty: undefined,
  files: [
    {
      path: "shop/checkout/total.rs",
      how: "M",
      hunks: [
        {
          actions: [
            {
              key: "comment",
              label: "comment",
              kind: "press",
              wire: { type: "button", onclick: still },
            },
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
  ],
};

const SIDES = [
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
] as const;

function Held() {
  return <p class="text-note text-text-faint">{WORDS.held}</p>;
}

export default function PageLookCases() {
  const lang = storeValue(ui().lang);
  const tree = () =>
    treeLookOf(
      {
        group: "appearance",
        beneath: BENEATH,
        folds: foldsOf("appearance", BENEATH),
        leaves: { buildings: [], record: [] },
        reading: null,
      },
      "fixture-tree",
      { words: (key) => say(lang(), key), pick: still, toggle: still, walk: still },
    );
  return (
    <>
      <GalleryCase label="refrain looks · a conflict bar, two lines, a version pair and the editor's box">
        <div class="flex flex-col gap-base">
          <Conflict
            text="Somebody else saved Plan.md while you were editing it."
            wire={{ role: "alert" }}
            actions={<span class="text-note">{WORDS.compare}</span>}
          />
          <Line tone="faint">
            <Held />
          </Line>
          <Line tone="alert">
            <Held />
          </Line>
          <Pair sides={SIDES} more={null} />
          <div class="flex h-output flex-col overflow-hidden rounded-control border border-edge-input">
            <EditorHost />
          </div>
        </div>
      </GalleryCase>
      <GalleryCase label="settings looks · the tree on the appearance group, a card saving, and a door key">
        <div class="flex items-start gap-base">
          <div class="w-index">
            <TreeControls {...tree()} />
          </div>
          <div class="flex min-w-0 flex-1 flex-col gap-base">
            <SaveCard
              title={say(lang(), "colours_tokens")}
              note={say(lang(), "colours_tokens_note")}
              settled={say(lang(), "saving_sent")}
              saveLabel={say(lang(), "colours_css_apply")}
              saving={sent("fixture")}
              onSave={still}
            >
              <Held />
            </SaveCard>
            <DoorKey
              glyph="check"
              label="open the door"
              note="for 12 hours"
              wire={{ type: "button", "aria-disabled": false, onclick: still }}
            />
          </div>
        </div>
        <PanelDialog titleId="fixture-panel-title" onClose={still} />
      </GalleryCase>
      <GalleryCase label="monitor code column · one file, one hunk with its presses, and the empty column">
        <div class="flex flex-col gap-base">
          <CodeColumnView look={DIFF} />
          <CodeColumnView look={{ empty: "Nothing changed yet.", files: [] }} />
        </div>
      </GalleryCase>
    </>
  );
}
