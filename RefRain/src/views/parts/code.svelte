<script lang="ts" module>
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  import type { Snippet } from "svelte";

  export interface CodeProps {
    readonly path: string;
    readonly text: string;
    readonly cited?: number | undefined;
    readonly aside?: Snippet | undefined;
  }
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { crumbsOf, gutterOf } from "./code";
  import Look from "./code.look.svelte";

  const { path, text, cited, aside }: CodeProps = $props();

  const { lang } = ui();

  let scroller = $state<HTMLElement | undefined>(undefined);
  let mark = $state<HTMLElement | undefined>(undefined);

  const holdScroller: Attachment<HTMLElement> = (node) => {
    scroller = node;
    return () => {
      scroller = undefined;
    };
  };
  const holdMark: Attachment<HTMLElement> = (node) => {
    mark = node;
    return () => {
      mark = undefined;
    };
  };

  $effect(() => {
    if (scroller === undefined || mark === undefined) return;
    scroller.scrollTop = Math.max(0, mark.offsetTop - scroller.clientHeight / 2);
  });
</script>

<Look
  crumbs={crumbsOf(path)}
  trail={{ "aria-label": say($lang, "code_crumbs") }}
  gutter={gutterOf(text)}
  {text}
  {path}
  {cited}
  copy={{ text, note: say($lang, "code_copy_note"), form: "bare" }}
  scroller={holdScroller}
  mark={holdMark}
  {aside}
/>
