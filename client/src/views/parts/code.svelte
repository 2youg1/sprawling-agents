<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A file the way a tool handed it over: which file it was, which line
// each row is, and its grammar's colours (`inked.svelte`).
//
// **Read-only by construction.** There is no editor here, no minimap
// and no language server: this view answers "what did that call
// produce", and every one of those three answers a question about
// editing a project instead.
//
// This is the seat: it decides the trail and the gutter, holds the two
// elements it scrolls, and draws whatever `./code.look.svelte` is.
import type { Snippet } from "svelte";

export interface CodeProps {
  // The file this came from, as the tool named it. An empty string
  // when the call named none: the trail is then not drawn and nothing
  // is coloured.
  readonly path: string;
  readonly text: string;
  // The line a reader was sent to, one-based: the view scrolls it to
  // the middle and marks it with the accent bar. The inspector's file
  // view names the line a call read from.
  readonly cited?: number | undefined;
  // Controls of the caller's that belong to this file, drawn at the
  // right end of the header.
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

  // The scroller is moved rather than the mark scrolled into view, which
  // would move every scrolling box around it as well - the page included.
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
