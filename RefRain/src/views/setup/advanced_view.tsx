// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal } from "solid-js";

import { EDITORS, type Editor, type Opening } from "../../core/editor";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Combobox from "../parts/combobox_view";
import Field from "../parts/field.solid";
import Admission from "../settings/admission_view";
import Card from "./card_view";
import { doorOf } from "./door";
import Door from "./door.look.solid";

const WORDS: Record<Editor, Key> = {
  none: "setup_editor_none",
  vscode: "setup_editor_vscode",
  "vscode-insiders": "setup_editor_vscode_insiders",
  vscodium: "setup_editor_vscodium",
  cursor: "setup_editor_cursor",
  windsurf: "setup_editor_windsurf",
  zed: "setup_editor_zed",
};

export default function AdvancedSection() {
  const u = ui();
  const lang = storeValue(u.lang);
  const [held, setHeld] = createSignal(u.prefs.editor());
  const word = (key: Key) => say(lang(), key);

  function keep(next: Pick<Opening, "editor" | "folder">): void {
    u.prefs.setEditor(next);
    setHeld(next);
  }

  function pick(value: string): void {
    const editor = EDITORS.find((each) => each === value);
    if (editor !== undefined) keep({ ...held(), editor });
  }

  return (
    <div class="grid grid-fit items-start gap-base">
      <Card title={word("setup_editor")} note={word("setup_editor_note")}>
        <Combobox
          label={word("setup_editor")}
          placeholder={word("part_search")}
          empty={word("part_no_match")}
          choices={EDITORS.map((editor) => ({ value: editor, label: word(WORDS[editor]) }))}
          value={held().editor}
          onPick={pick}
        />
        <Field
          label={word("setup_editor_folder")}
          help={word("setup_editor_folder_help")}
          value={held().folder}
          mono
          onInput={(folder) => keep({ ...held(), folder })}
        />
      </Card>
      <Admission />
      <Card title={word("privacy_open")} note={word("privacy_open_note")}>
        <Door {...doorOf({ kind: "setup", group: "privacy" }, word("setup_group_privacy"))} />
      </Card>
      <Door {...doorOf({ kind: "welcome" }, word("setup_rerun"))} />
    </div>
  );
}
