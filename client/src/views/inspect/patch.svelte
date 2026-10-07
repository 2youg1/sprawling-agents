<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // One file's patch between two checkpoints, as a reader points at it
  // (docs/frontend-method.md §7F, client/Spec.lean §7-2): a number column, a sign column and the line,
  // the added and removed lines on the accent and alert washes, and the
  // line a person last chose marked by a 2 px accent bar. The inspector's
  // diff and `changes.svelte`'s open row both draw a patch through this
  // file, so one reading of a diff exists.
  //
  // **One number per line, the one the line has in the tree it lives in**:
  // the new tree's for a kept or added line, the old tree's for a removed
  // one, so the column reads as the file reads. The quote a chosen line
  // puts in the conversation names the tree as well (`changes.ts`'s
  // `quoteLine`), which is where the two numbers a line could have are
  // told apart.
  //
  // **Lines do not fold.** A patch is read by column, so a long line runs
  // sideways inside the scroller and the rows stay one baseline step
  // apart.
  //
  // This file is the seat (client D95): it quotes a chosen line into the
  // conversation and draws whatever `./patch.look.svelte` is; the rows and
  // their wire bags are `./patch.ts`'s.
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, HunksAnswer } from "../../wire";
  import { quoteLine } from "../changes";
  import type { CodeLine } from "../changes";
  import { quoteInto } from "../talk/quoting";
  import { lookOf, type PatchLook } from "./patch";
  import Look from "./patch.look.svelte";

  interface Props {
    readonly patch: HunksAnswer;
    // The conversation a chosen line is quoted into; absent where the
    // page has no conversation, and then the numbers are not controls.
    readonly talk?: Address | undefined;
    // The patch line a person last chose, by its place in the patch.
    readonly cursor?: number | null;
    readonly onCursor?: (line: CodeLine) => void;
  }

  const { patch, talk, cursor = null, onCursor }: Props = $props();

  const u = ui();
  const lang = u.lang;

  // A chosen line joins whatever the person had already started to
  // write there - in the box when one is open there, in its draft
  // otherwise - and the link then opens that conversation.
  function choose(line: CodeLine): void {
    if (talk === undefined) return;
    quoteInto(u.prefs, talk, quoteLine(patch, line));
    onCursor?.(line);
  }

  const look: PatchLook = $derived(
    lookOf(
      patch,
      cursor,
      {
        withheld: (n, reason) => fill(say($lang, "run_withheld"), { n, reason }),
        folded: (n) => fill(say($lang, "change_folded"), { n }),
        quote: (n) => fill(say($lang, "change_line_quote"), { n }),
      },
      { href: talk === undefined ? undefined : toFragment({ kind: "talk", address: talk }), choose },
    ),
  );
</script>

<Look {...look} />
