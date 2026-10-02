// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The verdicts a proposal card sends are the one place a person's answer
// meets the city's grammar for it (documents D16): these tests hold the
// four spellings the city refuses out of every answer, and the order in
// which a sent answer learns its fate.

import { describe, expect, test } from "bun:test";

import { decideProposals } from "../../core/commands";
import type { AxError, ProposalCard, Slice } from "../../wire";
import { Address, B3Hash, RunId } from "../../wire";
import { advance, editOf, refusesCard, standingOf, takesAny, verdictsOf } from "./proposals";
import type { Deciding } from "./proposals";

const DOC = Address.make("shop/notes/plan.md");
const V1 = B3Hash.make("1".repeat(64));
const V2 = B3Hash.make("2".repeat(64));
const ID = B3Hash.make("c".repeat(64));

function slice(kind: Slice["kind"], text: string): Slice {
  return { kind, lead: "", text, trail: " " };
}

// A card that keeps its first sentence, swaps the second for a new one,
// and adds a third.
const CARD: ProposalCard = {
  id: ID,
  run: RunId.make("11111111-1111-4111-8111-111111111111"),
  baseline: V1,
  span: { start: 0, end: 40 },
  slices: [
    slice("same", "The reader opens a version."),
    slice("delete", "It edits in place."),
    slice("insert", "It edits in the browser."),
    slice("insert", "It saves the version's bytes."),
  ],
};

function refusal(action: string, subject: string): AxError {
  return { action, code: "E_VERSION_CONFLICT", gate: null, nearby: [], recovery: "reject the card", retry: "no", subject };
}

describe("the verdicts a card sends", () => {
  test("accepting the card whole names every change once and nothing unchanged", () => {
    expect(verdictsOf(CARD, null)).toEqual([
      { slice: 1, verdict: "accept" },
      { slice: 2, verdict: "accept" },
      { slice: 3, verdict: "accept" },
    ]);
  });

  test("the edit a person starts from sends what accepting whole sends", () => {
    expect(verdictsOf(CARD, editOf(CARD))).toEqual(verdictsOf(CARD, null));
  });

  test("an untaken sentence is left unnamed, and a rewritten insertion is amended", () => {
    const edit = new Map(editOf(CARD))
      .set(1, { taken: false, text: "It edits in place." })
      .set(2, { taken: true, text: "It edits in this browser." });
    expect(verdictsOf(CARD, edit)).toEqual([
      { slice: 2, verdict: { amend: { text: "It edits in this browser." } } },
      { slice: 3, verdict: "accept" },
    ]);
  });

  // A deletion has no words of its own to change: the city refuses an
  // amended deletion, so a changed text on one is never sent.
  test("a deletion is accepted, never amended, whatever its take holds", () => {
    const edit = new Map(editOf(CARD)).set(1, { taken: true, text: "something else" });
    expect(verdictsOf(CARD, edit)).toContainEqual({ slice: 1, verdict: "accept" });
  });

  test("a take on an unchanged sentence or a place off the card is not sent", () => {
    const edit = new Map(editOf(CARD))
      .set(0, { taken: true, text: "The reader opens a version." })
      .set(9, { taken: true, text: "nowhere" });
    expect(verdictsOf(CARD, edit).map((verdict) => verdict.slice)).toEqual([1, 2, 3]);
  });

  test("an edit that takes nothing takes nothing", () => {
    const none = new Map([...editOf(CARD)].map(([place, take]) => [place, { ...take, taken: false }]));
    expect(takesAny(none)).toBe(false);
    expect(verdictsOf(CARD, none)).toEqual([]);
  });
});

describe("whether a card can still be accepted", () => {
  test("a card on the version the document holds is current", () => {
    expect(standingOf(CARD, V1)).toEqual({ kind: "current" });
  });

  test("a card on another version, or on a document with none, is stale", () => {
    expect(standingOf(CARD, V2)).toEqual({ kind: "stale", now: V2 });
    expect(standingOf(CARD, null)).toEqual({ kind: "stale", now: null });
  });
});

describe("where a sent answer stands", () => {
  const command = decideProposals(DOC, [{ proposal: ID, verdicts: [] }]);
  const sent: Deciding = { kind: "sent", command };

  test("a link lost in flight waits, and sends the same command when it is back", () => {
    const pending = advance(sent, { kind: "lost" });
    expect(pending).toEqual({ kind: "pending", command });
    expect(advance(pending, { kind: "relinked" })).toEqual(sent);
  });

  test("a refusal ends a decision in flight and nothing at rest", () => {
    const error = refusal("decide proposals", `proposal ${ID} was made on version ${V1}`);
    expect(advance(sent, { kind: "refusal", error })).toEqual({ kind: "refused", error });
    expect(advance({ kind: "idle" }, { kind: "refusal", error })).toEqual({ kind: "idle" });
  });

  test("a refusal is this card's when it names this card, no card, or the document", () => {
    const other = B3Hash.make("d".repeat(64));
    expect(refusesCard(refusal("decide proposals", `proposal ${ID} is named twice`), DOC, ID)).toBe(true);
    expect(refusesCard(refusal("decide a proposal", "sentence 4 is not on this card"), DOC, ID)).toBe(true);
    expect(refusesCard(refusal("save a document", DOC), DOC, ID)).toBe(true);
    expect(refusesCard(refusal("decide proposals", `proposal ${other} was decided already`), DOC, ID)).toBe(false);
    expect(refusesCard(refusal("save a document", "shop/notes/other.md"), DOC, ID)).toBe(false);
  });
});
