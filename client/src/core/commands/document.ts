// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two commands that write a document, apart from `commands.ts` for
// the length budget that file sits at.

import { mintIdem } from "../idem";
import type { Address, B3Hash, Command, ProposalDecision, TextEdit } from "../../wire";

// A save of one document: edits made on the version `baseline`, each a
// byte span of that version and the text that replaces it. The city
// refuses it once the version has moved, and the draft stays on the
// page; the receipt is the `document_written` line that carries this
// command's key (`crates/wire/Spec.lean` §8-72).
export function putRange(doc: Address, baseline: B3Hash, edits: readonly TextEdit[]): Command {
  return { put_range: { doc, baseline, edits, idem: mintIdem() } };
}

// A person's decision on proposal cards of one document: a changed
// sentence left unnamed is rejected, so a card with no verdicts is
// rejected whole. What is accepted lands as one save (`crates/wire/Spec.lean` §8-73).
export function decideProposals(doc: Address, decisions: readonly ProposalDecision[]): Command {
  return { decide_proposals: { doc, decisions, idem: mintIdem() } };
}
