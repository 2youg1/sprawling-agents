// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which documents the mailbox asks about, and how many cards are open on
// them (client/Spec.lean §4-55). The mailbox key counts them and the deciding
// section lists them, so both read this one reading: two counts of the
// same cards would be the first thing to disagree.
//
// The city answers open cards one document at a time, and no question
// lists the documents that carry them; the documents asked about are
// the ones a `proposal_offered` line this page folded named
// (`core/belief/proposed.ts`), newest offer first.

import { derived } from "svelte/store";
import type { Readable } from "svelte/store";

import type { Asking } from "../../core/asking";
import type { Proposed } from "../../core/belief/proposed";
import type { Address } from "../../wire";

// A reader derives this from `belief.proposed` alone, whose identity
// moves only when an offer is folded, so the questions below are not
// asked afresh on every record the belief takes.
export function proposedDocs(proposed: Proposed): Address[] {
  return [...proposed].sort(([, left], [, right]) => right - left).map(([doc]) => doc);
}

// How many cards are open on these documents, as the city last answered
// for each; a document not yet answered counts none.
export function openCards(asking: Asking, docs: readonly Address[]): Readable<number> {
  return derived(
    docs.map((doc) => asking.ask({ proposals: doc })),
    (answers) => answers.reduce((sum, answer) => sum + (answer !== undefined && "proposals" in answer ? answer.proposals.open.length : 0), 0),
  );
}
