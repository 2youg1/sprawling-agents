<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
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
    readonly path: string;

    readonly line: number;
    readonly shown: Shown;
  }

  const { path, line, shown }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const reach = $derived(reachOf({ ...u.prefs.editor(), path, line }, shown));

  let copied = $state(false);

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
