<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // Every version the city knows of this document, newest first, and the
  // diff between any two (client/Spec.lean §4-46, §4-61): the list is the
  // city's `Versions` answer, with the draft as one more side. A version
  // whose text this page already held is compared from that text; any
  // other is gathered window by window by its version. The comparison is
  // a read-only view from the editor's chunk, built again whenever
  // either side changes, and it can be exported as a Markdown file.
  import { untrack } from "svelte";

  import { readAnswer } from "../../core/answered";
  import { positionsOf } from "../../core/document_pos";
  import { recorded, whole } from "../../core/document_windows";
  import { fill, say } from "../../core/lang";
  import { isoDay, isoTime, kib } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, DocumentVersion } from "../../wire";
  import Button from "../parts/button.svelte";
  import Empty from "../parts/empty.svelte";
  import { editorWire } from "./editor";
  import Editor from "./editor.look.svelte";
  import { Gathered } from "./gathered.svelte";
  import { sideWire, type PairLook } from "./pair";
  import Pair from "./pair.look.svelte";
  import type { Session } from "./session.svelte";
  import { short } from "./reading";
  import { saveFile } from "./saved_file";

  interface Props {
    readonly at: Address;
    readonly session: Session;
    readonly label: string;
    readonly phrases: Readonly<Record<string, string>>;
  }

  const { at, session, label, phrases }: Props = $props();

  const u = ui();
  const { lang } = u;

  const DRAFT = "draft";

  const asked = $derived(u.conn.asking.ask({ versions: { at } }));
  const read = $derived(readAnswer($asked, (answer) => ("versions" in answer ? answer.versions : undefined)));
  // A version written twice is listed once, where it was written last.
  const listed = $derived(
    read.kind === "held"
      ? read.value.versions.filter((each, index, all) => all.findIndex((other) => other.version === each.version) === index)
      : [],
  );

  function sourceOf(each: DocumentVersion): string {
    const source = each.source;
    if (source === "on_disk") return say($lang, "refrain_version_on_disk");
    if ("before" in source) return say($lang, "refrain_version_outside");
    return `${say($lang, "refrain_version_saved")} · ${isoDay(source.saved.at)} ${isoTime(source.saved.at)}`;
  }

  function nameOf(each: DocumentVersion): string {
    const parts = [short(each.version), sourceOf(each), ...(each.bytes === undefined || each.bytes === null ? [] : [kib(each.bytes)])];
    return each.kept ? parts.join(" · ") : `${parts.join(" · ")} · ${say($lang, "refrain_version_not_kept")}`;
  }

  // The draft is the editor's text when the reading opens, not as it is
  // typed: a comparison of a moving text would redraw on every key.
  const draft = $derived(
    session.receipt.kind === "clean" || session.receipt.kind === "saved" ? null : (session.editing?.text() ?? null),
  );
  const sides = $derived([
    ...(draft === null ? [] : [{ value: DRAFT, label: say($lang, "refrain_version_draft"), kept: true }]),
    ...listed.map((each) => ({ value: each.version, label: nameOf(each), kept: each.kept })),
  ]);
  const choosable = $derived(sides.filter((each) => each.kept));

  let from = $state<string | null>(null);
  let to = $state<string | null>(null);
  const right = $derived(to ?? choosable[0]?.value ?? null);
  const left = $derived(from ?? choosable.find((each) => each.value !== right)?.value ?? null);

  function pickFrom(value: string): void {
    from = value;
  }

  function pickTo(value: string): void {
    to = value;
  }

  // The two lists, from and to, each offering every side.
  const pair: PairLook = $derived({
    sides: [
      { key: "from", label: say($lang, "refrain_from"), held: left ?? "", pick: pickFrom },
      { key: "to", label: say($lang, "refrain_to"), held: right ?? "", pick: pickTo },
    ].map((side) => ({
      key: side.key,
      label: side.label,
      held: side.held,
      options: sides.map((each) => ({ value: each.value, label: each.label, disabled: !each.kept })),
      wire: sideWire(side.label, side.pick),
    })),
    more: read.kind === "held" && read.value.more ? say($lang, "refrain_versions_more") : null,
  });

  const format = $derived(session.file?.kind === "text" ? session.file.gathering.format : "plain");
  const leftText = new Gathered(u.conn.asking);
  const rightText = new Gathered(u.conn.asking);

  // The text of one side: the draft, a version this page held, or the
  // version gathered by its digest; null while it is gathered.
  function textOf(value: string | null, gathered: Gathered): string | null {
    if (value === null) return null;
    if (value === DRAFT) return draft;
    const held = session.versions.find((each) => each.version === value);
    if (held !== undefined) return held.text;
    const got = gathered.value;
    if (got?.version !== value || !whole(got)) return null;
    return positionsOf(got.version, got.encoding, got.text).editor;
  }

  $effect(() => {
    for (const [value, gathered] of [[left, leftText], [right, rightText]] as const) {
      const listedVersion = listed.find((each) => each.version === value)?.version;
      const held = session.versions.some((each) => each.version === value);
      const wanted = listedVersion === undefined || held ? null : listedVersion;
      if (wanted !== untrack(() => gathered.value?.version ?? null)) {
        gathered.start(wanted === null ? null : recorded(wanted, format));
      }
    }
  });

  const a = $derived(textOf(left, leftText));
  const b = $derived(textOf(right, rightText));
  const lost = $derived(leftText.lost !== null || rightText.lost !== null);
  const nameOfSide = (value: string | null): string => sides.find((each) => each.value === value)?.label ?? "";

  let host = $state<HTMLDivElement>();
  const wire = editorWire((box) => {
    host = box;
  });

  $effect(() => {
    const parent = host;
    const older = a;
    const newer = b;
    if (parent === undefined || older === null || newer === null) return;
    let shown: { readonly destroy: () => void } | null = null;
    let gone = false;
    void import("./editing").then(({ openComparison }) => {
      if (!gone) shown = openComparison(parent, { from: older, to: newer }, label, phrases);
    });
    return () => {
      gone = true;
      shown?.destroy();
    };
  });

  function exportComparison(): void {
    if (a === null || b === null) return;
    const comparison = {
      name: label,
      from: { label: nameOfSide(left), text: a },
      to: { label: nameOfSide(right), text: b },
      about: [say($lang, "refrain_compared_source")],
    };
    void import("./formats/compared").then(({ comparedMarkdown, comparedName }) => {
      saveFile(comparedName(label), "text/markdown", comparedMarkdown($lang, comparison));
    });
  }
</script>

{#if read.kind === "asking"}
  <p class="p-pane text-note text-text-faint">{say($lang, "refrain_versions_asking")}</p>
{:else if choosable.length < 2}
  <Empty missing="refrain_versions_one" />
{:else}
  <Pair {...pair} />
  {#if lost}
    <p class="p-pane text-note text-text-quiet">{say($lang, "refrain_version_lost")}</p>
  {:else if a === null || b === null}
    <p class="p-pane text-note text-text-faint">{say($lang, "refrain_version_reading")}</p>
  {:else if a === b}
    <p class="p-pane text-note text-text-quiet">{fill(say($lang, "refrain_diff_same"), { version: nameOfSide(left) })}</p>
  {:else}
    <div class="flex justify-end border-b border-edge px-wide py-tight">
      <Button label={say($lang, "refrain_export_comparison")} tone="quiet" onPress={exportComparison} />
    </div>
    <Editor {wire} />
  {/if}
{/if}
