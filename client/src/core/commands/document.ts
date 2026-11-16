// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { mintIdem } from "../idem";
import type { Address, B3Hash, Command, ProposalDecision, TextEdit } from "../../wire";

export function putRange(doc: Address, baseline: B3Hash, edits: readonly TextEdit[]): Command {
  return { put_range: { doc, baseline, edits, idem: mintIdem() } };
}

export function decideProposals(doc: Address, decisions: readonly ProposalDecision[]): Command {
  return { decide_proposals: { doc, decisions, idem: mintIdem() } };
}
