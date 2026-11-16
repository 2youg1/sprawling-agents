// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
