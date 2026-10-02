<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A call that changed a file, as the patch of that file between the two
  // checkpoints around it (client-SPEC 4-45, 7F). The checkpoint after the
  // call is the tree the patch reads into, so the patch covers the whole
  // wave the call was part of, and the line above it names both trees.
  //
  // **A line number of a past tree is not offered to an editor.** The
  // page asks what moved between the later checkpoint and the worktree;
  // when this file is among it, the worktree no longer holds the text the
  // numbers belong to, and the line is offered as `path:line` to copy
  // rather than as a link that would open on a different line (4-39).
</script>

<script lang="ts">
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";

  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, Answer, Call, Query, RoundsAnswer } from "../../wire";
  import { shortOid } from "../changes";
  import type { CodeLine } from "../changes";
  import { bracketOf } from "../checkpoints";
  import Unanswered from "../parts/unanswered.svelte";
  import Crumb from "./crumb.svelte";
  import Patch from "./patch.svelte";

  interface Props {
    readonly call: Call;
    readonly rounds: RoundsAnswer;
    readonly talk: Address;
  }

  const { call, rounds, talk }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const NOTHING: Readable<Answer | undefined> = readable(undefined);

  // The file the call changed: where its registration located the change,
  // or the path it was called on.
  const path = $derived.by((): string => {
    const render = call.render;
    const located = render !== null && render !== undefined && typeof render === "object" ? render.diff.locations[0] : undefined;
    return located ?? call.subject ?? "";
  });
  const bracket = $derived(bracketOf(rounds, call.at));

  const question = $derived.by((): Query | null => {
    const { base, head } = bracket;
    return base === null || head === null || path === "" ? null : { hunks: { oid_a: base, oid_b: head, path } };
  });
  const asked = $derived(question === null ? NOTHING : u.conn.asking.ask(question));
  const read = $derived(readAnswer($asked, (held) => ("hunks" in held ? held.hunks : undefined)));

  // What has moved since the later checkpoint, the worktree being the
  // other end.
  const sinceQuestion = $derived.by((): Query | null => (bracket.head === null ? null : { changes: { base: bracket.head, head: null } }));
  const since = $derived(sinceQuestion === null ? NOTHING : u.conn.asking.ask(sinceQuestion));
  const moved = $derived(readAnswer($since, (held) => ("changes" in held ? held.changes.files : undefined)));
  const shown = $derived(moved.kind === "held" && !moved.value.some((file) => file.path === path) ? "current" : "past");

  let cursor = $state<CodeLine | null>(null);
  const line = $derived.by((): number => {
    const chosen = cursor;
    if (chosen === null) return 1;
    return (chosen.kind === "removed" ? chosen.old : chosen.new) ?? 1;
  });
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <Crumb {path} {line} {shown}>
    {#if bracket.base !== null && bracket.head !== null}
      <span class="font-mono">{shortOid(bracket.head)} ← {shortOid(bracket.base)}</span>
    {/if}
  </Crumb>
  <div class="min-h-0 flex-1 overflow-auto bg-page">
    {#if question === null}
      <p class="px-wide py-snug text-note text-text-faint">{say($lang, "run_patch_needs_checkpoint")}</p>
    {:else if read.kind === "unavailable"}
      <div class="px-wide py-snug"><Unanswered query={read.query} asked={question} /></div>
    {:else if read.kind === "held"}
      <Patch
        patch={read.value}
        {talk}
        cursor={cursor?.number ?? null}
        onCursor={(chosen) => {
          cursor = chosen;
        }}
      />
    {:else}
      <p class="px-wide py-snug text-note text-text-faint">…</p>
    {/if}
  </div>
</div>
