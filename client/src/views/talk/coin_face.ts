// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Send and stop as two faces of one key, the way a player's switch is
// play on one side and stop on the other (client/Spec.lean §7I, client D18).
//
// Which face is up is decided here and nowhere else: words in the box
// turn the send face up, and a run in front of the person with an empty
// box turns the stop face up; with neither, the send face is up and
// faint, and pressing it does nothing. The two are never offered
// together, so a hand that reaches for this key in a hurry gets the one
// thing its face says.
export type Face = "send" | "stop" | "idle";

export function faceOf(words: string, running: boolean): Face {
  if (words.trim() !== "") return "send";
  return running ? "stop" : "idle";
}
