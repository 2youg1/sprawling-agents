// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every icon in the client is one row of this table: one name per
// drawing, one SVG path per name. `glyph.svelte` is the only thing that
// draws them, so a shape cannot appear in two slightly different
// versions in two files. The city illustrations are drawings rather
// than icons and stay where they are; this table is for the marks that
// name a screen, an action, or a state.
//
// Every drawing lives on one twenty-square in one stroke: 1.5 units
// wide, round caps, round joins, no fill. A circle is written as its
// two arcs and a second element in a drawing is a second subpath of the
// same string, so one name is always exactly one `d`.
//
// The second half of this file is the closed set of states a thing can
// be in, and the one mapping from a state to the drawing that tells it
// apart and the paint tier that only repeats it. A chip and a dot both
// ask this mapping, so the same state draws the same shape wherever it
// stands.

// The navigation marks name the screen they open; the rest name what
// the control does or what the mark stands for.
export type GlyphName =
  // A conversation with whoever lives in a room.
  | "talk"
  // The skyline: every building the city keeps.
  | "city"
  // The ledger as a bound document.
  | "record"
  // The line of what has been spent.
  | "cost"
  // The two sliders of the settings screen.
  | "setup"
  // A hand raised: something waits for a person. The one drawing both
  // the rail and the `waiting` state use.
  | "hand"
  // The magnifier over everything the palette can reach.
  | "search"
  // The tree's disclosure arrow; the class beside it turns it.
  | "chevron"
  // A folder with an arrow leaving it: reveal the path in the file
  // manager.
  | "reveal"
  // An empty circle: nothing in particular is happening.
  | "ring"
  // The heartbeat line: something is going on right now.
  | "pulse"
  // Two crossed strokes: refused.
  | "cross"
  // One stroke that lands: done.
  | "check"
  // A wrench: a tool is at work.
  | "tool"
  // A square, the stop key of every player: stop what is going, and the
  // mark of a run somebody stopped, so the control and its outcome read
  // as one shape.
  | "stop"
  // A line that runs into a wall: it ran out of what it was allowed.
  | "capped"
  // An arrow rising from the box: send what is written.
  | "send";

export const GLYPHS: Record<GlyphName, string> = {
  talk: "M3 5.5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H9l-4 3v-3H5a2 2 0 0 1-2-2z",
  city: "M3 17V8l4-2v11M7 17V4l5 2v11M12 17V9l5-2v10M2.5 17h15",
  record: "M5 3h10v14H5zM8 7h4M8 10h4M8 13h2",
  cost: "M3 16l4-6 3 3 4-7 3 4M3 17h14",
  setup:
    "M3 6h8M14 6h3M3 14h3M9 14h8M14 6a2 2 0 1 1-4 0 2 2 0 1 1 4 0M9 14a2 2 0 1 1-4 0 2 2 0 1 1 4 0",
  hand: "M6 10V4.5a1.5 1.5 0 0 1 3 0V9M9 4a1.5 1.5 0 0 1 3 0v5M12 5a1.5 1.5 0 0 1 3 0v6.5c0 3-2 5.5-5 5.5s-4.5-2-6-4.5L3 11a1.4 1.4 0 0 1 2.3-1.5L6 10.5",
  search: "M14 9a5 5 0 1 1-10 0 5 5 0 1 1 10 0M13 13l4 4",
  chevron: "M7 4l6 6-6 6",
  reveal: "M2 15.5v-11h5l2 2.5h9v8.5zM8 12l5.5-4.5M10 7.5h3.5V11",
  ring: "M14 10a4 4 0 1 1-8 0 4 4 0 1 1 8 0",
  pulse: "M2.5 10.5h3l2.5-5.5 3.5 10 2.5-4.5h3.5",
  cross: "M5.5 5.5l9 9M14.5 5.5l-9 9",
  check: "M4 10.5l4 4 8-9",
  tool: "M13.5 3a3.5 3.5 0 0 0-3.3 4.7L3.5 14.4l2.1 2.1 6.7-6.7A3.5 3.5 0 0 0 17 6.5l-2.2.7-2-2 .7-2.2z",
  stop: "M6 6h8v8H6z",
  capped: "M3 10h10M10 7l3 3-3 3M16 4v12",
  send: "M10 16V4M5 9l5-5 5 5",
};

// The three paint tiers a mark may take (client-SPEC 4-32). They are
// named here because `statusLook` below picks one per state and
// `badge.svelte` paints them; the spelling of the three lives in this
// one line.
export type Weight = "quiet" | "live" | "alert";

// The closed set of states a thing the client draws can be in. A new
// state is a design decision, not an extension: it owes the table below
// one row and this union one member at the same time.
export type Status = "idle" | "live" | "waiting" | "refused" | "done";

export interface StatusLook {
  // The encoding: five different drawings, so the state survives a
  // forced-colours mode that repaints every fill and border.
  readonly glyph: GlyphName;
  // The reinforcement: which tier merely repeats what the drawing
  // already says. Two states share a tier on purpose - `waiting` and
  // `refused` both mean a person must act, `idle` and `done` both mean
  // nothing is moving - and the drawing is what tells them apart.
  readonly weight: Weight;
}

const STATUS: Record<Status, StatusLook> = {
  idle: { glyph: "ring", weight: "quiet" },
  live: { glyph: "pulse", weight: "live" },
  waiting: { glyph: "hand", weight: "alert" },
  refused: { glyph: "cross", weight: "alert" },
  done: { glyph: "check", weight: "quiet" },
};

// How one of the five states is told: its drawing and its paint tier.
export function statusLook(state: Status): StatusLook {
  return STATUS[state];
}
