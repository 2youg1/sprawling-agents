// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a person's answer to one proposal card sends, and where the
// answer stands until the card is gone (client-SPEC 4-55,
// `crates/wire/Spec.lean` §8-73).
//
// **The verdicts are spelled here and nowhere else.** The city refuses
// a verdict on a sentence that did not change, a verdict on a place the
// card does not have, a sentence named twice, and an amended deletion
// (documents D16); every answer a card gives is built by `verdictsOf`,
// which can spell none of the four.

import type { Address, AxError, B3Hash, Command, ProposalCard, ProposalDecision, SliceVerdict } from "../../wire";

// ------------------------------------------------------------ standing

// Whether a card can still be accepted: it can when it was made on the
// version the document holds now. A stale card can only be rejected,
// because the city judges an accepting decision against the version and
// a rejection against nothing.
export type Standing =
  | { readonly kind: "current" }
  // `now` is the version the document holds, or `null` when it holds
  // none (missing or unreadable).
  | { readonly kind: "stale"; readonly now: B3Hash | null };

export function standingOf(card: ProposalCard, version: B3Hash | null): Standing {
  return card.baseline === version ? { kind: "current" } : { kind: "stale", now: version };
}

// ------------------------------------------------------------ the edit

// What a person made of one changed sentence: whether they take it, and
// for an inserted sentence the words it lands as.
export interface Take {
  readonly taken: boolean;
  readonly text: string;
}

// A card under edit: one take per changed sentence, by its place on the
// card counted from zero. Unchanged sentences have no take, so no edit
// can name one.
export type Edit = ReadonlyMap<number, Take>;

// The edit a person starts from: every change taken as it was proposed.
export function editOf(card: ProposalCard): Edit {
  return new Map(
    card.slices.flatMap((slice, place) => (slice.kind === "same" ? [] : [[place, { taken: true, text: slice.text }]])),
  );
}

// The edit after a person changed one take; a place the edit does not
// hold is a sentence that did not change, and stays without one.
export function retaken(edit: Edit, place: number, change: Partial<Take>): Edit {
  const take = edit.get(place);
  return take === undefined ? edit : new Map(edit).set(place, { ...take, ...change });
}

// Whether an edit accepts anything; one that takes nothing is a
// rejection, and a person rejects with the rejection's own answer.
export function takesAny(edit: Edit): boolean {
  return [...edit.values()].some((take) => take.taken);
}

// The verdicts an answer sends. `null` accepts the card whole; an edit
// names every sentence it takes, as `amend` when an inserted sentence's
// words were changed. A take on a place the card does not change is
// dropped rather than sent, because the city refuses the whole decision
// for it.
export function verdictsOf(card: ProposalCard, edit: Edit | null): SliceVerdict[] {
  return card.slices.flatMap((slice, place): SliceVerdict[] => {
    const take = edit === null ? { taken: true, text: slice.text } : edit.get(place);
    if (slice.kind === "same" || !take?.taken) return [];
    if (slice.kind === "insert" && take.text !== slice.text) {
      return [{ slice: place, verdict: { amend: { text: take.text } } }];
    }
    return [{ slice: place, verdict: "accept" }];
  });
}

// The decision a rejection sends: the card named with no verdicts.
export function rejectionOf(card: ProposalCard): ProposalDecision {
  return { proposal: card.id, verdicts: [] };
}

// ------------------------------------------------------------ deciding

// Where a sent answer stands. A decision that lands closes the card, and
// the card leaving the city's answer is the receipt, so there is no
// `landed` here: the card is simply no longer drawn.
export type Deciding =
  | { readonly kind: "idle" }
  | { readonly kind: "sent"; readonly command: Command }
  // The link dropped before the city answered; the same command, key
  // included, goes again when it comes back, and the city answers a key
  // it has seen with its first result.
  | { readonly kind: "pending"; readonly command: Command }
  | { readonly kind: "refused"; readonly error: AxError };

export type Happened =
  | { readonly kind: "sent"; readonly command: Command }
  | { readonly kind: "lost" }
  | { readonly kind: "relinked" }
  | { readonly kind: "refusal"; readonly error: AxError };

export function advance(deciding: Deciding, happened: Happened): Deciding {
  switch (happened.kind) {
    case "sent":
      return { kind: "sent", command: happened.command };
    case "lost":
      return deciding.kind === "sent" ? { kind: "pending", command: deciding.command } : deciding;
    case "relinked":
      return deciding.kind === "pending" ? { kind: "sent", command: deciding.command } : deciding;
    case "refusal":
      return deciding.kind === "sent" || deciding.kind === "pending"
        ? { kind: "refused", error: happened.error }
        : deciding;
  }
}

// The city's spellings of the actions a decision is refused at: the
// decision as a whole, and one sentence of it
// (`crates/documents/src/proposal.rs`). A refusal carries no key (§8-2),
// so these and the subject are how one is told to be this card's.
const DECIDING_ACTIONS: ReadonlySet<string> = new Set(["decide proposals", "decide a proposal"]);

// The digest-shaped word a refusal's subject names a card by.
const NAMES_A_CARD = /\b[0-9a-f]{64}\b/u;

// Whether a refusal answers a decision this page sent about `card` on
// `doc`: a decision refusal that names this card or no card at all, or
// a refusal of the document itself (a path the city will not write).
export function refusesCard(error: AxError, doc: Address, card: B3Hash): boolean {
  if (error.subject === doc) return true;
  if (!DECIDING_ACTIONS.has(error.action)) return false;
  return error.subject.includes(card) || !NAMES_A_CARD.test(error.subject);
}
