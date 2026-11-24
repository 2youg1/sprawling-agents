// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal } from "solid-js";
import { say } from "../../../core/lang";
import { ui } from "../../../ui";
import Combobox from "../../parts/combobox_view";
import { GalleryCase } from "../case";
import { CHOSEN, MODELS } from "../served";
import { storeValue } from "../../../core/solid_store";

export default function ComboboxCases() {
  const lang = storeValue(ui().lang);
  const [model, setModel] = createSignal<string | null>(null);
  const [held, setHeld] = createSignal<string | null>(MODELS.at(-1)?.id ?? CHOSEN.id);
  const [edge, setEdge] = createSignal<string | null>(CHOSEN.id);
  const [face, setFace] = createSignal<string | null>("Georgia");
  const choices = MODELS.map((each) => ({ value: each.id, label: each.id, note: each.context }));
  const faces = [
    { value: "Georgia", label: "Georgia", face: '"Georgia", ui-serif, serif' },
    { value: "Courier New", label: "Courier New", face: '"Courier New", ui-monospace, monospace' },
    {
      value: "Trebuchet MS",
      label: "Trebuchet MS",
      face: '"Trebuchet MS", ui-sans-serif, sans-serif',
    },
  ];
  return (
    <>
      <GalleryCase label="combobox · open on click, nothing chosen">
        <div class="flex flex-col gap-base pb-output">
          <Combobox
            label={say(lang(), "setup_main")}
            placeholder={say(lang(), "part_search")}
            empty={say(lang(), "part_no_match")}
            choices={choices}
            value={model()}
            onPick={setModel}
            starts="open"
          />
          <Combobox
            label={say(lang(), "setup_digest")}
            placeholder={say(lang(), "part_search")}
            empty={say(lang(), "part_no_match")}
            choices={choices}
            value={model()}
            onPick={setModel}
          />
        </div>
      </GalleryCase>
      <GalleryCase label="combobox · a value in force, the cursor on another row">
        <div class="pb-output">
          <Combobox
            label={say(lang(), "setup_main")}
            placeholder={say(lang(), "part_search")}
            empty={say(lang(), "part_no_match")}
            choices={choices}
            value={held()}
            onPick={setHeld}
            starts="open"
          />
        </div>
      </GalleryCase>
      <GalleryCase label="combobox · in the bottom corner of a box that clips it, opening upward">
        <div class="flex h-palette flex-col items-end justify-end overflow-hidden">
          <div class="w-tree">
            <Combobox
              label={say(lang(), "setup_main")}
              placeholder={say(lang(), "part_search")}
              empty={say(lang(), "part_no_match")}
              choices={choices}
              value={edge()}
              onPick={setEdge}
              starts="open"
            />
          </div>
        </div>
      </GalleryCase>
      <GalleryCase label="combobox · each row wears the face it names">
        <div class="pb-output">
          <Combobox
            label={say(lang(), "appearance_local")}
            placeholder={say(lang(), "part_search")}
            empty={say(lang(), "part_no_match")}
            choices={faces}
            value={face()}
            onPick={setFace}
            starts="open"
          />
        </div>
      </GalleryCase>
    </>
  );
}
