// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One file of the city as the pages that show it draw it: the text of
// its first window, how long the file is, whether it is text at all, and
// whether the window left some of it out. The city answers a document as
// a version (wire-SPEC 8-69); this is the one place a page turns that
// answer into what it draws, so the three pages that read a file agree.

import { readAnswer, type Answered } from "./answered";
import type { Answer, DocumentAnswer } from "../wire";

export interface DocumentText {
  readonly text: string;
  readonly bytes: number;
  readonly binary: boolean;
  readonly truncated: boolean;
}

export function readDocument(answer: Answer | undefined): Answered<DocumentText> {
  const read = readAnswer(answer, (held) => ("document" in held ? held.document : undefined));
  return read.kind === "held" ? documentText(read.value) : read;
}

// A file that is not there and one that will not read are both "the
// city could not show this file", named by the question a person asked;
// an empty file is a file with no text.
function documentText(document: DocumentAnswer): Answered<DocumentText> {
  const state = document.state;
  if (state === "missing" || "unreadable" in state) {
    return { kind: "unavailable", query: `Document(${document.at})` };
  }
  if ("empty" in state) {
    return { kind: "held", value: { text: "", bytes: 0, binary: false, truncated: false } };
  }
  const held = state.held;
  if (held.body === "opaque") {
    return { kind: "held", value: { text: "", bytes: held.bytes, binary: true, truncated: false } };
  }
  const body = held.body.text;
  return {
    kind: "held",
    value: { text: body.head.text, bytes: held.bytes, binary: false, truncated: body.coverage === "head" },
  };
}
