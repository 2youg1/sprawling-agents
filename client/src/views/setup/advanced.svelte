<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The advanced group: the way back into the welcome walk, and the
  // editor a link in the monitor opens a file in.
  //
  // The editor and the city's folder are kept by this browser, because
  // they are facts of the machine it runs on (client-SPEC 4-39). The
  // page only writes links; the browser hands each one to the editor
  // this machine registered for the scheme.

  import { EDITORS } from "../../core/editor";
  import type { Editor, Opening } from "../../core/editor";
  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import Field from "../parts/field.svelte";
  import Segmented from "../parts/segmented.svelte";

  // One word per editor, keyed by the editor: an editor added to
  // `core/editor.ts` leaves this table refusing to compile until it has
  // a name.
  const WORDS: Record<Editor, Key> = {
    none: "setup_editor_none",
    vscode: "setup_editor_vscode",
    "vscode-insiders": "setup_editor_vscode_insiders",
  };

  const { lang, prefs } = ui();

  let held = $state(prefs.editor());

  function keep(next: Pick<Opening, "editor" | "folder">): void {
    prefs.setEditor(next);
    held = next;
  }
</script>

<div class="flex flex-col gap-wide">
  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "setup_editor")}</span>
    <p class="text-note text-text-faint">{say($lang, "setup_editor_note")}</p>
    <Segmented
      label={say($lang, "setup_editor")}
      options={EDITORS.map((each) => ({ value: each, label: say($lang, WORDS[each]) }))}
      held={held.editor}
      onPick={(editor) => {
        keep({ ...held, editor });
      }}
    />
    <Field
      label={say($lang, "setup_editor_folder")}
      help={say($lang, "setup_editor_folder_help")}
      value={held.folder}
      mono
      onInput={(folder) => {
        keep({ ...held, folder });
      }}
    />
  </div>
  <!-- The door back into the welcome walk: a link, because it moves the
      address bar like every other way off this page. -->
  <a
    href={toFragment({ kind: "welcome" })}
    class="inline-flex h-control w-fit items-center rounded-control bg-raised px-base text-label hover:bg-raised-hover"
  >
    {say($lang, "setup_rerun")}
  </a>
</div>
