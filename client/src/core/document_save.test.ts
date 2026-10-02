// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, B3Hash, RunId, Seq, TimeMs } from "../wire";
import type { AxCode, AxError, EventRecord } from "../wire";
import { positionsOf } from "./document_pos";
import { advance, draftPlace, readDraft, receiptIn, saveOf, writeDraft } from "./document_save";
import type { Happened, Receipt, Sent } from "./document_save";

const DOC = Address.make("shop/notes.md");
const V = B3Hash.make("a".repeat(64));
const W = B3Hash.make("b".repeat(64));

function sentOf(): Sent {
  return saveOf(DOC, positionsOf(V, "utf8", "one\ntwo"), [{ from: 4, to: 7, insert: "2" }]);
}

function idemOf(sent: Sent): string {
  return "put_range" in sent.command ? sent.command.put_range.idem : "";
}

function written(idem: string, version: string): EventRecord {
  return {
    run: RunId.make("00000000-0000-0000-0000-000000000000"),
    seq: Seq.make(7),
    kind: "document_written",
    t: TimeMs.make(7),
    who: "city",
    prev: B3Hash.make("0".repeat(64)),
    v: 1,
    data: { at: DOC, baseline: V, version, bytes: 5, idem },
  };
}

function refusal(code: AxCode, action: string): AxError {
  return { action, code, nearby: [], recovery: "read the document again", retry: "no", subject: "x" };
}

describe("drafts", () => {
  test("keep a document's changes against its version, and read them back", () => {
    const draft = { version: V, changes: [{ from: 1, to: 2, insert: "é\n" }, { from: 5, to: 5, insert: "x" }] };
    expect([draftPlace(DOC), readDraft(writeDraft(draft), 9), writeDraft({ version: V, changes: [] })]).toEqual([
      "document:shop/notes.md", draft, "",
    ]);
  });

  test("refuse a stored value that is not a draft of a version this long", () => {
    const stored = (changes: unknown): string => JSON.stringify({ version: V, changes });
    expect([
      readDraft("not json", 9),
      readDraft(stored([{ from: 1, to: 12, insert: "" }]), 9),
      readDraft(stored([{ from: 4, to: 6, insert: "" }, { from: 5, to: 7, insert: "" }]), 9),
      readDraft(stored([{ from: 3, to: 2, insert: "" }]), 9),
      readDraft(JSON.stringify({ version: "v1", changes: [] }), 9),
    ]).toEqual([null, null, null, null, null]);
  });
});

describe("saveOf", () => {
  test("sends the changes as bytes of the baseline it was made on", () => {
    const sent = sentOf();
    expect({
      doc: sent.doc,
      baseline: sent.baseline,
      edits: "put_range" in sent.command ? sent.command.put_range.edits : [],
    }).toEqual({ doc: DOC, baseline: V, edits: [{ span: { start: 4, end: 7 }, text: "2" }] });
  });
});

describe("receiptIn", () => {
  test("reads the version only from the line that carries this save's key", () => {
    const sent = sentOf();
    expect([
      receiptIn([written("idem1-" + "0".repeat(32), W)], sent),
      receiptIn([written(idemOf(sent), "not a version")], sent),
      receiptIn([written(idemOf(sent), W)], sent),
    ]).toEqual([null, null, W]);
  });
});

describe("advance", () => {
  const sent = sentOf();
  const saving: Receipt = { kind: "saving", sent, edited: false };
  const steps: readonly (readonly [Receipt, Happened, Receipt])[] = [
    [{ kind: "clean" }, { kind: "edited", empty: false }, { kind: "draft" }],
    [{ kind: "draft" }, { kind: "edited", empty: true }, { kind: "clean" }],
    [{ kind: "draft" }, { kind: "sent", sent }, saving],
    [{ kind: "draft" }, { kind: "moved", version: W }, { kind: "conflict", current: W }],
    [{ kind: "clean" }, { kind: "moved", version: W }, { kind: "clean" }],
    [saving, { kind: "edited", empty: false }, { kind: "saving", sent, edited: true }],
    [saving, { kind: "landed", version: W }, { kind: "saved", version: W }],
    [{ kind: "saving", sent, edited: true }, { kind: "landed", version: W }, { kind: "draft" }],
    [saving, { kind: "moved", version: W }, saving],
    [saving, { kind: "lost" }, { kind: "pending", sent, edited: false }],
    [{ kind: "pending", sent, edited: false }, { kind: "relinked" }, saving],
    [{ kind: "pending", sent, edited: false }, { kind: "landed", version: W }, { kind: "saved", version: W }],
    [saving, { kind: "refusal", error: refusal("E_VERSION_CONFLICT", "save a document") }, { kind: "conflict", current: null }],
    [
      saving,
      { kind: "refusal", error: refusal("E_INVALID_ARGS", "save a document") },
      { kind: "refused", error: refusal("E_INVALID_ARGS", "save a document") },
    ],
    [saving, { kind: "refusal", error: refusal("E_INVALID_ARGS", "dispatch a task") }, saving],
    [{ kind: "draft" }, { kind: "refusal", error: refusal("E_VERSION_CONFLICT", "save a document") }, { kind: "draft" }],
    [{ kind: "refused", error: refusal("E_INVALID_ARGS", "save a document") }, { kind: "edited", empty: false }, { kind: "draft" }],
    [{ kind: "conflict", current: W }, { kind: "edited", empty: false }, { kind: "conflict", current: W }],
    [{ kind: "conflict", current: W }, { kind: "based", empty: false }, { kind: "draft" }],
    [{ kind: "conflict", current: W }, { kind: "based", empty: true }, { kind: "clean" }],
    [{ kind: "saved", version: W }, { kind: "edited", empty: false }, { kind: "draft" }],
  ];

  test("moves a save through its states on what happened, and only there", () => {
    expect(steps.map(([from, happened]) => advance(from, happened))).toEqual(steps.map(([, , to]) => to));
  });
});
