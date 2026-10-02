<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One document in RefRain: the head line, the reason the text is read
  // only when it is, the conflict bar, and the four readings. The editor
  // is mounted once and hidden rather than unmounted while the preview
  // or the versions are drawn, so the cursor, the selection, the undo
  // history and an input method's composition all survive a change of
  // reading (client/Spec.lean §7N).
  import { draftPlace } from "../../core/document_save";
  import { fill, say } from "../../core/lang";
  import { kib } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, B3Hash, ProposalCard } from "../../wire";
  import Empty from "../parts/empty.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import Unkept from "../parts/unkept.svelte";
  import Conflict from "./conflict.svelte";
  import { drawnAs } from "./formats/format";
  import HtmlPreview from "./formats/html.svelte";
  import Opaque from "./formats/opaque.svelte";
  import Head from "./head.svelte";
  import Preview from "./preview.svelte";
  import Proposals from "./proposals.svelte";
  import { Session } from "./session.svelte";
  import type { Locked } from "./session.svelte";
  import { phrasesIn, short, type Reading } from "./reading";
  import Versions from "./versions.svelte";

  interface Props {
    readonly at: Address;
    readonly building: Address;
    readonly version: B3Hash | null;
    // The reading it opens in; a person starts in the source.
    readonly reading?: Reading;
  }

  const { at, building, version, reading: first = "source" }: Props = $props();

  const u = ui();
  const lang = u.lang;
  // svelte-ignore state_referenced_locally (a session belongs to one document and version; refrain.svelte keys this component on both)
  const session = new Session(u, at, version);
  // Whether the browser refused to keep this document's draft (4-63).
  // svelte-ignore state_referenced_locally (one document per component, as above)
  const unkept = u.prefs.draftUnkept(draftPlace(at));
  const drafted = $derived(session.receipt.kind !== "clean" && session.receipt.kind !== "saved");

  // svelte-ignore state_referenced_locally (the first reading only; the person picks the rest)
  let reading = $state<Reading>(first);
  // The byte the next reading should open at, carried across a switch.
  let anchor = $state<number | null>(null);
  let top = 0;

  const name = $derived(at.slice(at.lastIndexOf("/") + 1));
  const markdown = $derived(session.file?.kind === "text" && session.file.gathering.format === "markdown");
  // An HTML text is previewed by this browser rather than by the city
  // (client/Spec.lean §4-54).
  const html = $derived(session.file?.kind === "text" && drawnAs(at) === "html");

  const phrases = $derived(phrasesIn($lang));

  // Switching reading keeps the place: the source's cursor becomes the
  // preview's first block, and the preview's first block the cursor.
  function pick(next: Reading): void {
    const editing = session.editing;
    const base = session.positions;
    if (editing !== null && base !== null) {
      // Only a Markdown preview has blocks to carry a place to and from.
      if (markdown && (reading === "source" || reading === "diff") && next === "preview") {
        anchor = base.bytes(editing.toBaseline(editing.cursor()));
      } else if (markdown && reading === "preview" && (next === "source" || next === "diff")) {
        editing.reveal(editing.fromBaseline(base.editorAt(top)));
      }
      editing.diffing(next === "diff");
    }
    reading = next;
    if (next === "source" || next === "diff") queueMicrotask(() => editing?.measure());
  }

  function lockedLine(locked: Locked): string {
    switch (locked.kind) {
      case "gathering":
        return fill(say($lang, "refrain_gathering"), { through: kib(locked.through), total: kib(locked.total) });
      case "too_large":
        return fill(say($lang, "refrain_too_large"), { through: kib(locked.through), total: kib(locked.total) });
      case "recorded":
        return fill(say($lang, "refrain_recorded"), { version: short(locked.version) });
      case "lost":
        return fill(say($lang, "refrain_lost"), { version: short(locked.version) });
    }
  }

  const editorShown = $derived(reading === "source" || reading === "diff");

  // A proposal card's stretch is a byte span of its version, so the
  // editor can be taken there only while it holds that version.
  function unshowable(card: ProposalCard): string | undefined {
    return session.editing !== null && session.positions?.version === card.baseline
      ? undefined
      : say($lang, "proposal_show_why");
  }

  function show(card: ProposalCard): void {
    const editing = session.editing;
    const base = session.positions;
    if (editing === null || base === null) return;
    if (!editorShown) pick("source");
    editing.reveal(editing.fromBaseline(base.editorAt(card.span.start)));
    editing.focus();
  }
</script>

<div class="refrain flex min-h-0 flex-1 flex-col bg-page">
  <Head
    {at}
    {building}
    {session}
    {reading}
    previewed={markdown || html}
    onPick={pick}
  />
  {#if session.locked !== null && session.file?.kind === "text"}
    <p class="refrain-line text-note text-text-faint">{lockedLine(session.locked)}</p>
  {/if}
  {#if session.receipt.kind === "refused"}
    <p class="refrain-line text-note text-alert">{session.receipt.error.recovery}</p>
  {:else if session.receipt.kind === "pending"}
    <p class="refrain-line text-note text-text-faint">{say($lang, "refrain_receipt_pending_why")}</p>
  {/if}
  {#if $unkept && drafted}
    <div class="refrain-line"><Unkept words={() => session.editing?.text() ?? ""} /></div>
  {/if}
  {#if session.lostDraft !== null}
    <p class="refrain-line text-note text-alert">
      {fill(say($lang, "refrain_draft_lost"), { version: short(session.lostDraft) })}
    </p>
  {/if}
  {#if reading === "diff" && session.positions !== null && (session.receipt.kind === "clean" || session.receipt.kind === "saved")}
    <p class="refrain-line text-note text-text-faint">
      {fill(say($lang, "refrain_diff_same"), { version: short(session.positions.version) })}
    </p>
  {/if}
  {#if session.receipt.kind === "conflict"}
    <Conflict
      {name}
      {session}
      onCompare={() => {
        pick("versions");
      }}
    />
  {/if}
  <Proposals doc={at} {unshowable} onShow={show} />
  {#if session.read.kind === "asking"}
    <p class="p-pane text-text-faint">…</p>
  {:else if session.read.kind === "unavailable"}
    <div class="p-pane">
      <Unanswered query={session.read.query} asked={{ document: { at } }} />
    </div>
  {:else if session.file?.kind === "missing"}
    <Empty missing="file_missing" />
  {:else if session.file?.kind === "unreadable"}
    <p class="p-pane text-note text-text-quiet">{fill(say($lang, "refrain_unreadable"), { reason: session.file.reason })}</p>
  {:else if session.file?.kind === "opaque"}
    <Opaque path={at} version={session.file.version} bytes={session.file.bytes} />
  {:else if session.opening === null}
    <p class="p-pane text-text-faint">…</p>
  {:else}
    <div class={["min-h-0 flex-1 flex-col", editorShown ? "flex" : "hidden"]}>
      {#await import("./source.svelte") then { default: Source }}
        <Source {session} label={name} {phrases} />
      {/await}
    </div>
    {#if reading === "preview" && html && session.positions !== null}
      <HtmlPreview
        text={session.editing?.text() ?? session.positions.editor}
        {drafted}
        {name}
      />
    {:else if reading === "preview" && session.positions !== null}
      <Preview
        positions={session.positions}
        {building}
        {at}
        {drafted}
        {anchor}
        onTop={(byte) => {
          top = byte;
          anchor = null;
        }}
      />
    {:else if reading === "versions"}
      <Versions {at} {session} label={name} {phrases} />
    {/if}
  {/if}
</div>
