

<script lang="ts">
  
  
  import { EDITORS } from "../../core/editor";
  import type { Editor, Opening } from "../../core/editor";
  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { ui } from "../../ui";
  import Combobox from "../parts/combobox.svelte";
  import Field from "../parts/field.svelte";
  import Admission from "../settings/admission.svelte";
  import Card from "./card.look.svelte";
  import { doorOf } from "./door";
  import Door from "./door.look.svelte";

  
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

  
  function pick(word: string): void {
    const editor = EDITORS.find((each) => each === word);
    if (editor !== undefined) keep({ ...held, editor });
  }
</script>

<div class="grid grid-fit items-start gap-base">
  <Card title={say($lang, "setup_editor")} note={say($lang, "setup_editor_note")}>
    <Combobox
      label={say($lang, "setup_editor")}
      placeholder={say($lang, "part_search")}
      empty={say($lang, "part_no_match")}
      choices={EDITORS.map((each) => ({ value: each, label: say($lang, WORDS[each]) }))}
      value={held.editor}
      onPick={pick}
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
  </Card>
  <Admission />
  
  <Card title={say($lang, "privacy_open")} note={say($lang, "privacy_open_note")}>
    <Door {...doorOf({ kind: "setup", group: "privacy" }, say($lang, "setup_group_privacy"))} />
  </Card>
  
  <Door {...doorOf({ kind: "welcome" }, say($lang, "setup_rerun"))} />
</div>
