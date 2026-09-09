// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Reading and writing frames. The generated `Schema` is the only reader:
// a frame this build cannot parse is `null`, and the link treats it as
// a closed connection, because the two ends disagree about the wire and
// the machine already knows what to do about that. This is the one
// place Effect is used at run time.

import { Either, Schema } from "effect";

import type { ClientFrame } from "../wire";
import { ServerFrame } from "../wire";

const readServerFrame = Schema.decodeUnknownEither(
  Schema.parseJson(ServerFrame),
);

export function decodeFrame(text: string): ServerFrame | null {
  const read = readServerFrame(text);
  return Either.isRight(read) ? read.right : null;
}

// Every frame this client builds is serialisable: nothing in the type
// carries a credential, so `JSON.stringify` cannot fail on it.
export function encodeFrame(frame: ClientFrame): string {
  return JSON.stringify(frame);
}
