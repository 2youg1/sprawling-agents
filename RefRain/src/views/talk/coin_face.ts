// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type Face = "send" | "stop" | "idle";

export function faceOf(words: string, running: boolean): Face {
  if (words.trim() !== "") return "send";
  return running ? "stop" : "idle";
}

export type Sent = "words" | "cancel" | "nothing";

export function pressCoin(face: Face): Sent {
  switch (face) {
    case "send":
      return "words";
    case "idle":
      return "nothing";
    case "stop":
      return "cancel";
  }
}
