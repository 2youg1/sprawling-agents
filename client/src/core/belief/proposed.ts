// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The documents a run offered a change to while this page listened,
// each with the ledger position of the newest offer (client-SPEC 4-55).
//
// **It records documents, not cards.** A card's identity is a digest the
// city computes (documents D13), and the lines that close a card carry
// that identity and not the document, so a page folding them could not
// tell which document lost a card. Which cards are still open is
// `Query::Proposals(document)`'s answer; this index only says which
// documents are worth asking about.

import type { Address, EventRecord, Seq } from "../../wire";

export type Proposed = ReadonlyMap<Address, Seq>;

export function proposedWith(held: Proposed, record: EventRecord): [Proposed, string | null] {
  void record;
  return [held, null];
}
