// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which request shapes a known host answers in, as one table and one
// reading of it. This is the only place the client states which faces
// a host speaks: `form.svelte` reads it to refuse a cell before the
// click rather than after a 404, and nothing else consults it.
//
// **The table says nothing about what the city will do.** The city
// fills a face's path and makes the call (`gateway::provider::preset`
// on that side); this table only stops a person choosing a shape the
// provider at a known host has never answered in. A host that is not
// in the table is a host this client knows nothing about, and every
// cell stays open - the widest judgement, never a narrower one
// (client-SPEC 4-25).
//
// The entries are addresses, not words: no language translates them,
// and the wire values name shapes the two languages spell the same.

import type { WireApi } from "../../../core/commands";

// One host and the faces it answers in, in the order the control
// offers them.
export const FACES: readonly (readonly [string, readonly WireApi[]])[] = [
  ["api.deepseek.com", ["chat"]],
  ["api.anthropic.com", ["messages"]],
  ["api.openai.com", ["responses", "chat"]],
  // Only the OpenAI-compatible chat face; the rest of this host speaks
  // Gemini's own shape, which the city does not write.
  ["generativelanguage.googleapis.com", ["chat"]],
];

// The faces a host answers in, or nothing when this table has no row
// for it. The host arrives as `hostOf` spelled it; DNS names answer
// the same in either case, so the reading folds case here.
export function facesOf(host: string | null): readonly WireApi[] | null {
  if (host === null) return null;
  const folded = host.toLowerCase();
  const found = FACES.find(([name]) => name === folded);
  return found === undefined ? null : found[1];
}
