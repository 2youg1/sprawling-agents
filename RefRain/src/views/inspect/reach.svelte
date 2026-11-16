<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The way from one line of the inspector to the person's own editor
  // (client/Spec.lean §4-39): a link when the line belongs to the worktree's
  // current text and an editor is chosen, and otherwise the location
  // written out to copy, so the page never claims an editor opened
  // something (`core/editor.ts`'s `reachOf`). It is drawn by the
  // inspector's worded key, `./text_key.look.svelte`.

  // How long the copy receipt holds: long enough to see, short enough
  // that it never becomes the control's face.
  const RECEIPT_MS = 1200;
</script>

<script lang="ts">
  import { reachOf } from "../../core/editor";
  import type { Shown } from "../../core/editor";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { TextKeyLook } from "./text_key";
  import Look from "./text_key.look.svelte";

  interface Props {
    // Relative to the city, as the ledger records it.
    readonly path: string;
    // One-based.
    readonly line: number;
    readonly shown: Shown;
  }

  const { path, line, shown }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const reach = $derived(reachOf({ ...u.prefs.editor(), path, line }, shown));

  let copied = $state(false);

  // The receipt appears only after the write landed, so a press that
  // quietly failed shows no receipt rather than a lying one.
  function copy(text: string): void {
    void navigator.clipboard.writeText(text).then(() => {
      copied = true;
      setTimeout(() => {
        copied = false;
      }, RECEIPT_MS);
    });
  }

  const look: TextKeyLook = $derived.by(() => {
    switch (reach.kind) {
      case "link":
        return { label: say($lang, "setup_editor"), wire: { href: reach.href } };
      case "copy": {
        const place = reach.text;
        return {
          label: say($lang, copied ? "code_copied" : "inspect_copy_place"),
          wire: {
            type: "button",
            onclick: () => {
              copy(place);
            },
          },
        };
      }
    }
  });
</script>

<Look {...look} />
