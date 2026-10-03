// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the earlier stretch of a room is drawn, the one answer both
// modes of the conversation read (client D80): the whole thread folds
// it behind the divider, results only folds it behind its heading, and
// both open it while the session now open holds no run of its own -
// a session opened by a model change or `/new` would otherwise leave
// "everything" drawing one folded line over an empty page.
export type EarlierDrawn = "none" | "folded" | "open";

export function earlierDrawn(shown: number, earlier: number): EarlierDrawn {
  if (earlier === 0) return "none";
  return shown === 0 ? "open" : "folded";
}
