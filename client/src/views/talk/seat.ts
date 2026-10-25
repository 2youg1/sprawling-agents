// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where a conversation column stands, which decides the two facts it
// draws or leaves to a neighbour (client/Spec.lean §7K, "one place on a
// screen"): the address line, which every tier draws in the box's
// placeholder or at a past session's head (4-10a), and the frozen facts
// of the session - model, effort, mode, policy, sandbox - which the first
// message head states unless the workbench's gauge above the column
// already does (docs/frontend-method.md §7D).
//
// `page` is the conversation on the zen and blend page; `workbench` is the
// same column under the panorama workbench's chosen session; `specimen` is
// a `#/gallery` fixture, which draws a thread with no address line,
// because the fixture is not the page and names no room a palette could
// leave.
export type Seat = "page" | "workbench" | "specimen";

export function drawsAddress(seat: Seat): boolean {
  switch (seat) {
    case "page":
    case "workbench":
      return true;
    case "specimen":
      return false;
  }
}

// Where a run's frozen facts are said: on its first head, or elsewhere -
// on the head of the run that opened the stretch, or in the gauge.
export type FrozenFacts = "head" | "elsewhere";

// The frozen facts of the run at `at` in the stretch on screen.
export function factsAt(seat: Seat, at: number): FrozenFacts {
  return at === 0 && seat !== "workbench" ? "head" : "elsewhere";
}
