// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export function copiedSource(selected: string, drawn: string, source: string): string | null {
  const chosen = squeezed(selected);
  return chosen !== "" && chosen === squeezed(drawn) ? source : null;
}

function squeezed(text: string): string {
  return text.replace(/\s+/gu, "");
}
