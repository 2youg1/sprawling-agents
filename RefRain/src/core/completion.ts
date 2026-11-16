// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What Tab makes of a typed line against the verb table. Its own
// module because the question is its own: the table says what a verb
// is, the parser says what a line is, and this says what the gap
// between a prefix and its matches is worth.

import { SLASH, parse } from "./slash";

function shared(spellings: readonly string[]): string {
  const first = spellings.at(0);
  if (first === undefined) {
    return "";
  }
  let out = first;
  for (const spelling of spellings) {
    while (!spelling.startsWith(out)) {
      out = out.slice(0, -1);
    }
  }
  return out;
}

// The one match completed, or the longest prefix every match shares.
// When several match and their shared prefix is already typed, the
// verb the list's cursor points at is taken instead: a second Tab on
// `/st` gives the row the person is looking at, where the prefix alone
// would leave Tab doing nothing. A line Tab cannot improve comes back
// unchanged, so the caller has nothing to decide.
export function completed(line: string, pointed?: string): string {
  const call = parse(line);
  if (call === null) {
    return line;
  }
  const hits = SLASH.filter((known) => known.spelling.startsWith(call.verb));
  const only = hits.length === 1 ? hits.at(0) : undefined;
  if (only !== undefined) {
    return call.rest === "" ? `${only.spelling} ` : line;
  }
  const prefix = shared(hits.map((known) => known.spelling));
  if (prefix.length > call.verb.length) {
    return prefix;
  }
  const taken = hits.find((known) => known.spelling === pointed);
  return taken !== undefined && call.rest === "" ? `${taken.spelling} ` : line;
}
