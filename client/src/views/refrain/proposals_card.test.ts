// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The card's look value is all a look draws, so these tests hold for
// any look: which box each changed sentence carries and
// what pressing it changes, the words a screen reader hears, and why an
// answer cannot be given now.

import { describe, expect, test } from "bun:test";

import { decideProposals } from "../../core/commands";
import { fill, say } from "../../core/lang";
import type { AxError, ProposalCard, Slice } from "../../wire";
import { Address, B3Hash, RunId } from "../../wire";
import { editOf, rejectionOf, retaken } from "./proposals";
import type { Deciding, Edit, Standing, Take } from "./proposals";
import { cardLookOf, refusalsOf } from "./proposals_card";
import type { RowLook } from "./proposals_card";

const DOC = Address.make("shop/notes/plan.md");
const V1 = B3Hash.make("1".repeat(64));
const V2 = B3Hash.make("2".repeat(64));

function slice(kind: Slice["kind"], text: string, trail = " "): Slice {
  return { kind, lead: "", text, trail };
}

// A card that keeps its first sentence and swaps the second for a new
// one with no space between the two.
const CARD: ProposalCard = {
  id: B3Hash.make("c".repeat(64)),
  run: RunId.make("11111111-1111-4111-8111-111111111111"),
  baseline: V1,
  span: { start: 0, end: 40 },
  slices: [
    slice("same", "The reader opens a version."),
    slice("delete", "It edits in place.", ""),
    slice("insert", "It edits in the browser and keeps a draft per version."),
  ],
};

const CURRENT: Standing = { kind: "current" };
const IDLE: Deciding = { kind: "idle" };

// A recorder for the seat's one hand.
function recorder(): { readonly calls: [number, Partial<Take>][]; readonly retake: (place: number, change: Partial<Take>) => void } {
  const calls: [number, Partial<Take>][] = [];
  return {
    calls,
    retake: (place, change) => {
      calls.push([place, change]);
    },
  };
}

function rowsOf(edit: Edit, hands = recorder()): readonly RowLook[] {
  const { body } = cardLookOf(CARD, { standing: CURRENT, deciding: IDLE, edit, lead: undefined }, "en", hands);
  return body.kind === "edit" ? body.rows : [];
}

// The struck row and the written row of the card, which has one of each.
function changed(rows: readonly RowLook[]): {
  readonly struck: Extract<RowLook, { kind: "struck" }> | undefined;
  readonly written: Extract<RowLook, { kind: "written" }> | undefined;
} {
  return {
    struck: rows.flatMap((row) => (row.kind === "struck" ? [row] : []))[0],
    written: rows.flatMap((row) => (row.kind === "written" ? [row] : []))[0],
  };
}

describe("the card under edit", () => {
  test("an unchanged sentence has no box, a removal a take box, an insertion a take box and its words", () => {
    const rows = rowsOf(editOf(CARD));
    expect(rows.map((row) => row.kind)).toEqual(["kept", "struck", "written"]);
    const { struck, written } = changed(rows);
    expect(struck?.take["aria-label"]).toBe(fill(say("en", "proposal_take"), { words: "It edits in place." }));
    expect(written?.take["aria-label"]).toBe(fill(say("en", "proposal_take"), { words: "It edits in the browser and…" }));
    expect(written?.words["aria-label"]).toBe(say("en", "proposal_amend"));
    expect(written?.words.value).toBe("It edits in the browser and keeps a draft per version.");
  });

  test("a take box and a text box each change their own sentence's take", () => {
    const hands = recorder();
    const { struck, written } = changed(rowsOf(editOf(CARD), hands));
    struck?.take.onchange({ currentTarget: { checked: false } });
    written?.words.oninput({ currentTarget: { value: "It edits in the browser." } });
    expect(hands.calls).toEqual([
      [1, { taken: false }],
      [2, { text: "It edits in the browser." }],
    ]);
  });

  test("a sentence left out is drawn as left out, with its box unchecked", () => {
    const [, struck] = rowsOf(retaken(editOf(CARD), 1, { taken: false }));
    expect(struck).toMatchObject({ kind: "struck", taken: false, take: { checked: false } });
  });
});

describe("the card as its diff", () => {
  test("each changed sentence is announced by its kind, and an insertion touching a removal stands apart", () => {
    const { body } = cardLookOf(CARD, { standing: CURRENT, deciding: IDLE, edit: null, lead: undefined }, "en", recorder());
    expect((body.kind === "diff" ? body.pieces : []).map(({ kind, said, abuts }) => ({ kind, said, abuts }))).toEqual([
      { kind: "same", said: undefined, abuts: false },
      { kind: "delete", said: say("en", "proposal_removed"), abuts: false },
      { kind: "insert", said: say("en", "proposal_added"), abuts: true },
    ]);
  });

  test("where a sent decision stands is said in the card's live region", () => {
    const sent: Deciding = { kind: "sent", command: decideProposals(DOC, [rejectionOf(CARD)]) };
    const error: AxError = { action: "decide proposals", code: "E_VERSION_CONFLICT", gate: null, nearby: [], recovery: "read it again", retry: "no", subject: DOC };
    const statusOf = (deciding: Deciding): unknown =>
      cardLookOf(CARD, { standing: CURRENT, deciding, edit: null, lead: undefined }, "en", recorder()).status;
    expect(statusOf(IDLE)).toBeUndefined();
    expect(statusOf(sent)).toEqual({ tone: "faint", text: say("en", "proposal_deciding") });
    expect(statusOf({ kind: "refused", error })).toEqual({ tone: "alert", text: "read it again" });
  });
});

describe("why an answer cannot be given now", () => {
  test("a stale card can only be rejected, and says so on accept and edit", () => {
    const stale = say("en", "proposal_stale_why");
    expect(refusalsOf({ standing: { kind: "stale", now: V2 }, deciding: IDLE, edit: null }, "en")).toEqual({
      accept: stale,
      edit: stale,
      reject: undefined,
    });
  });

  test("a card in flight answers nothing", () => {
    const busy = say("en", "proposal_busy_why");
    const sent: Deciding = { kind: "sent", command: decideProposals(DOC, [rejectionOf(CARD)]) };
    expect(refusalsOf({ standing: CURRENT, deciding: sent, edit: null }, "en")).toEqual({ accept: busy, edit: busy, reject: busy });
  });

  test("an edit that takes nothing is not sent as an acceptance", () => {
    const none = retaken(retaken(editOf(CARD), 1, { taken: false }), 2, { taken: false });
    expect(refusalsOf({ standing: CURRENT, deciding: IDLE, edit: none }, "en").accept).toBe(say("en", "proposal_nothing_taken"));
    expect(refusalsOf({ standing: CURRENT, deciding: IDLE, edit: editOf(CARD) }, "en").accept).toBeUndefined();
  });
});
