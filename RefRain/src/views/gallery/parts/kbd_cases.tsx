// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";

import { LABELS } from "../../../core/keys";
import { say } from "../../../core/lang";
import type { Key, Lang } from "../../../core/lang";
import { KbdMarks } from "../../parts/kbd.look";
import { chordMarks } from "../../parts/kbd_marks";
import type { KbdProps } from "../../parts/kbd_marks";
import { GalleryCase } from "../case";

const ROWS: readonly { readonly word: Key; readonly key: KbdProps }[] = [
  { word: LABELS.palette, key: { action: "palette" } },
  { word: "part_save", key: { move: "line.next" } },
  { word: "part_delete", key: { initial: "d" } },
];

export function KbdCases(props: { readonly lang: Lang }) {
  return (
    <GalleryCase label="kbd · a chord, a line move and a first letter at a row's end">
      <ul class="flex flex-col">
        <For each={ROWS}>
          {(row) => (
            <li class="flex items-center justify-between gap-base py-tight text-label">
              <span class="text-text-quiet">{say(props.lang, row.word)}</span>
              <KbdMarks marks={chordMarks(row.key)} />
            </li>
          )}
        </For>
      </ul>
    </GalleryCase>
  );
}
