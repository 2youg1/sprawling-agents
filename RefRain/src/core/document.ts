// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

function documentText(document: DocumentAnswer): Answered<DocumentText> {
  const state = document.state;
  const asked = `Document(${document.at})`;
  if (state === "missing") {
    return { kind: "unavailable", query: asked, reason: null };
  }
  if ("unreadable" in state) {
    return { kind: "unavailable", query: asked, reason: state.unreadable.reason };
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
    value: {
      text: body.head.text,
      bytes: held.bytes,
      binary: false,
      truncated: body.coverage === "head",
    },
  };
}
