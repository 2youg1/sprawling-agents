// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every icon in the client is one name in this file, and `glyph.svelte`
// is the only thing that draws them: it maps each name to one icon of
// the lucide set, so a shape cannot appear in two slightly different
// versions in two files, and a person meets the icons they already know
// from other software (docs/frontend-method.md §4-34). The city illustrations are
// drawings rather than icons and stay where they are; these are the
// marks that name a screen, an action, or a state.
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
  // A hand raised: something waits for a person. The one drawing the
  // city table and the runs board both use for it.
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
  | "send"
  // The three edge keys: the tiers of the world layer, the mailbox, and
  // the settings panel.
  | "layers"
  | "inbox"
  | "settings"
  // What bounds the run in this room: who answers the gate, and the
  // sandbox it is boxed in.
  | "gate"
  | "sandbox"
  // A prompt and its cursor: what a command printed, on the inspector's
  // tab of a terminal.
  | "terminal"
  // A page with a pen across it: a change a run proposes to a document,
  // waiting for the person to decide it.
  | "propose"
  // A pushpin: a session the person keeps above the others.
  | "pin"
  // Three dots in a row: the menu of what can be done to one row.
  | "more"
  // A line that splits in two: branch the conversation from this point.
  | "branch"
  // Two sheets, one over the other: put this text on the clipboard.
  | "copy"
  // A door standing open, and a key: open the remote door, and replace
  // the city key a paired device holds; closing it is the `gate` lock.
  | "door"
  | "key";

// The three paint tiers a mark may take (client/Spec.lean §4-32). They are
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
