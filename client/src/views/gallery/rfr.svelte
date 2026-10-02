<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // RefRain, the right side's document editor (client/Spec.lean §7N), on a
  // made-up city: a Markdown document in its source and in its preview,
  // a plain text file read literally (a byte-order mark, CRLF lines, a
  // tab), a file larger than RefRain edits, a missing file, and a draft
  // kept on a version the city has since replaced - drawn as its conflict
  // bar and as the two versions compared. Each is drawn at the width the
  // right side has beside a conversation at 1440.
  import { draftPlace, writeDraft } from "../../core/document_save";
  import { EDITABLE_BYTES_MAX } from "../../core/document_windows";
  import { Address, B3Hash, Seq } from "../../wire";
  import type { Answer, DocumentState, Query } from "../../wire";

  const SHOP = Address.make("shop");
  const NOTES = "notes/plan.md";
  const TEXT = "notes/log.txt";
  const LARGE = "notes/corpus.md";
  const MOVED = "notes/moved.md";

  const V1 = B3Hash.make("3c1f0a9e".repeat(8));
  const V2 = B3Hash.make("9d24be07".repeat(8));
  const V3 = B3Hash.make("51ab7c3e".repeat(8));
  const V4 = B3Hash.make("e0f17a42".repeat(8));
  const V0 = B3Hash.make("07c41e09".repeat(8));

  const MARKDOWN = [
    "# Plan",
    "",
    "The reader opens a **version**, edits it in the browser, and saves it as that version's bytes.",
    "",
    "- [x] read the whole text",
    "- [ ] save through `PutRange`",
    "",
    "| step | where |",
    "| :--- | ---: |",
    "| coordinates | `document_pos.ts` |",
    "",
    "```rust",
    "let applied = documents::save(source, baseline, &edits)?;",
    "```",
    "",
  ].join("\n");

  // wording-ok: fixture text is a file's contents, not a page's words
  const LOG = "\uFEFFstarted\r\n\tchecked 41 tests\r\nfinished\r\n";
  const BEFORE = "# Moved\n\nThe first line.\nThe second line.\n";
  const AFTER = "# Moved\n\nThe first line, rewritten by a resident.\nThe second line.\n";

  function bytes(text: string): number {
    return new TextEncoder().encode(text).length;
  }

  // A file the city holds: `total` bytes long, `text` being its first
  // window, which is the whole file when the coverage says so.
  function held(version: B3Hash, text: string, total: number, coverage: "whole" | "head"): DocumentState {
    const encoding = text.startsWith("\uFEFF") ? "utf8_bom" : "utf8";
    return {
      held: {
        version,
        format: text.startsWith("#") ? "markdown" : "plain",
        bytes: total,
        body: { text: { encoding, coverage, head: { span: { start: 0, end: bytes(text) }, text } } },
      },
    };
  }

  const STATES: Readonly<Record<string, DocumentState>> = {
    [`shop/${NOTES}`]: held(V1, MARKDOWN, bytes(MARKDOWN), "whole"),
    [`shop/${TEXT}`]: held(V2, LOG, bytes(LOG), "whole"),
    [`shop/${LARGE}`]: held(V3, MARKDOWN, EDITABLE_BYTES_MAX * 2, "head"),
    [`shop/${MOVED}`]: held(V4, AFTER, bytes(AFTER), "whole"),
  };

  // The answers this made-up city gives: each document above, the old
  // version of the moved one by range and in its list of versions, and
  // the preview of the plan as the city's Markdown grammar lays it out.
  function answering(query: Query): Answer | undefined {
    if (typeof query !== "object") return undefined;
    if ("document" in query) {
      const at = query.document.at;
      return { document: { at, state: STATES[at] ?? "missing" } };
    }
    if ("range" in query && query.range.version === V0) {
      const start = query.range.range.start;
      const window = start === 0 ? { span: { start: 0, end: bytes(BEFORE) }, text: BEFORE } : { span: { start, end: start }, text: "" };
      return { range: { version: V0, window } };
    }
    if ("versions" in query && query.versions.at === `shop/${MOVED}`) {
      const versions = [
        { version: V4, bytes: bytes(AFTER), kept: true, source: "on_disk" as const },
        { version: V0, bytes: bytes(BEFORE), kept: true, source: { before: { seq: Seq.make(7) } } },
      ];
      return { versions: { at: query.versions.at, versions, more: false } };
    }
    if ("preview" in query && query.preview.version === V1) {
      return { preview: { version: V1, preview: { laid: { span: { start: 0, end: bytes(MARKDOWN) }, blocks: PLAN } } } };
    }
    return undefined;
  }

  const span = (start: number, end: number): { start: number; end: number } => ({ start, end });
  const PLAN = [
    { heading: { level: 1, span: span(0, 7), inline: [{ text: "Plan" }] } },
    {
      paragraph: {
        span: span(8, 101),
        inline: [
          { text: "The reader opens a " },
          { strong: [{ text: "version" }] },
          { text: ", edits it in the browser, and saves it as that version's bytes." },
        ],
      },
    },
    {
      list: {
        span: span(102, 155),
        order: "bullet" as const,
        spacing: "tight" as const,
        items: [
          { span: span(102, 126), check: "done" as const, blocks: [{ paragraph: { span: span(108, 126), inline: [{ text: "read the whole text" }] } }] },
          {
            span: span(127, 155),
            check: "open" as const,
            blocks: [{ paragraph: { span: span(133, 155), inline: [{ text: "save through " }, { code: "PutRange" }] } }],
          },
        ],
      },
    },
    {
      table: {
        span: span(156, 213),
        align: ["left" as const, "right" as const],
        head: { cells: [[{ text: "step" }], [{ text: "where" }]] },
        body: [{ cells: [[{ text: "coordinates" }], [{ code: "document_pos.ts" }]] }],
      },
    },
    { code: { span: span(214, 287), info: "rust", text: "let applied = documents::save(source, baseline, &edits)?;\n" } },
  ];

  function keptDraft(): string {
    return writeDraft({ version: V0, changes: [{ from: 24, to: 24, insert: " Kept as a draft." }] });
  }

  const MOVED_PLACE = draftPlace(Address.make(`shop/${MOVED}`));
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Document from "../refrain/document.svelte";
  import RefRain from "../refrain/refrain.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  // The draft the conflict cases restore, written where RefRain looks
  // for it before they mount.
  ui().prefs.setDraft(MOVED_PLACE, keptDraft());

  const FRAME = "refrain-specimen flex h-[480px] flex-col overflow-hidden bg-page";
</script>

<Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering}>
  <Case label="refrain · Markdown, its source" width={600}>
    <div class={FRAME}><RefRain building={SHOP} path={NOTES} version={null} /></div>
  </Case>
  <Case label="refrain · Markdown, its preview" width={600}>
    <div class={FRAME}>
      <Document at={Address.make(`shop/${NOTES}`)} building={SHOP} version={null} reading="preview" />
    </div>
  </Case>
  <Case label="refrain · a text file read literally: a mark, CRLF lines, a tab" width={600}>
    <div class={FRAME}><RefRain building={SHOP} path={TEXT} version={null} /></div>
  </Case>
  <Case label="refrain · larger than RefRain edits, the first window read only" width={600}>
    <div class={FRAME}><RefRain building={SHOP} path={LARGE} version={null} /></div>
  </Case>
  <Case label="refrain · a missing file" width={600}>
    <div class={FRAME}><RefRain building={SHOP} path="notes/gone.md" version={null} /></div>
  </Case>
  <Case label="refrain · a draft on a version the city replaced" width={600}>
    <div class={FRAME}><RefRain building={SHOP} path={MOVED} version={null} /></div>
  </Case>
  <Case label="refrain · the replaced version against the city's" width={600}>
    <div class={FRAME}>
      <Document at={Address.make(`shop/${MOVED}`)} building={SHOP} version={null} reading="versions" />
    </div>
  </Case>
</Stand>
