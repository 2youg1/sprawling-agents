// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a copy out of a laid-out reply puts on the clipboard. A selection
// that holds the whole reply copies the Markdown the model wrote, because
// the text a browser reads off the drawn page has lost every heading
// mark, list bullet, emphasis and code fence. A selection of some of
// the reply is left to the browser: the person chose those words, and
// the city's block spans are relative to the question that read each
// stretch (`core/replying.ts`), so they cannot say which source bytes a
// part of the page was drawn from.
//
// The two texts are compared with their white space squeezed out,
// because how a browser spells the gaps between drawn blocks differs
// between engines and is not what the person selected.

export function copiedSource(selected: string, drawn: string, source: string): string | null {
  const chosen = squeezed(selected);
  return chosen !== "" && chosen === squeezed(drawn) ? source : null;
}

function squeezed(text: string): string {
  return text.replace(/\s+/gu, "");
}
