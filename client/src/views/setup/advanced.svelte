<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The advanced group, the developer's: the way back into the welcome
  // walk, the editor a link in the monitor opens a file in, and what
  // bounds a run's work (refrain roadmap Q6).
  //
  // The editor and the city's folder are kept by this browser, because
  // they are facts of the machine it runs on (client/Spec.lean §4-39). The
  // page only writes links; the browser hands each one to the editor
  // this machine registered for the scheme.

  import { EDITORS } from "../../core/editor";
  import type { Editor, Opening } from "../../core/editor";
  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import Field from "../parts/field.svelte";
  import Admission from "../settings/admission.svelte";

  // One word per editor, keyed by the editor: an editor added to
  // `core/editor.ts` leaves this table refusing to compile until it has
  // a name.
  const WORDS: Record<Editor, Key> = {
    none: "setup_editor_none",
    vscode: "setup_editor_vscode",
    "vscode-insiders": "setup_editor_vscode_insiders",
    vscodium: "setup_editor_vscodium",
    cursor: "setup_editor_cursor",
    windsurf: "setup_editor_windsurf",
    zed: "setup_editor_zed",
  };

  const { lang, prefs } = ui();

  let held = $state(prefs.editor());

  function keep(next: Pick<Opening, "editor" | "folder">): void {
    prefs.setEditor(next);
    held = next;
  }

  // A native list rather than a segmented track: seven editors are more
  // than a track of equal cells holds in a settings card, and a list
  // reads the same to a keyboard and a screen reader at any width.
  function pick(word: string): void {
    const editor = EDITORS.find((each) => each === word);
    if (editor !== undefined) keep({ ...held, editor });
  }
</script>

<div class="grid grid-fit items-start gap-base">
  <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "setup_editor")}</span>
    <p class="text-note text-text-faint">{say($lang, "setup_editor_note")}</p>
    <select
      class="h-control w-full min-w-0 rounded-control border border-edge-input bg-raised px-base text-body text-text"
      aria-label={say($lang, "setup_editor")}
      value={held.editor}
      onchange={(event) => {
        pick(event.currentTarget.value);
      }}
    >
      {#each EDITORS as each (each)}
        <option value={each}>{say($lang, WORDS[each])}</option>
      {/each}
    </select>
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
  <Admission />
  <!-- The door back into the welcome walk: a link, because it moves the
      address bar like every other way off this page. -->
  <a
    href={toFragment({ kind: "welcome" })}
    class="inline-flex h-control w-fit items-center rounded-control bg-raised px-base text-label hover:bg-raised-hover"
  >
    {say($lang, "setup_rerun")}
  </a>
</div>
