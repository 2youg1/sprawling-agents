// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the thread's notes give their looks (client D86): a note is a
// row of pieces - words in one of four inks, or a name that leads to a
// room or a session - and the seats build that row from what the city
// said, so `note.look.svelte` draws every note one way and a look put
// in its place is handed the same row. A resident's letter is built
// from the same pieces.

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { lasted } from "../../core/time";
import type { HandbackNote, ReplyEnd, ReplyEnded } from "../../wire";

// Which of the four inks a piece of a note is drawn in: the note's
// own faint ink, the quieter ink of a name, and the two that say how
// a thing ended.
export type Ink = "faint" | "quiet" | "accent" | "alert";

// A name or a path a note leads to: the person's words are set in the
// reading face, an address in the mono face.
export type Face = "text" | "mono";

export type Piece =
  | { readonly kind: "words"; readonly text: string; readonly ink: Ink }
  | { readonly kind: "link"; readonly text: string; readonly href: string; readonly face: Face };

// One note's row. `note` and `status` are the roles the row carries
// for a screen reader; a row inside a larger thing carries none.
export interface NoteLook {
  readonly role: "note" | "status" | undefined;
  readonly pieces: readonly Piece[];
}

// A resident's letter: its accessible name, the head row and the words.
export interface LetterLook {
  readonly label: string;
  readonly head: readonly Piece[];
  readonly said: string;
}

// Where a note sits in the thread's column. A turn spaces its children
// `gap-snug` apart, eight pixels, so a one-line note keeps eight pixels
// of its own and a shape - a bubble, a letter, a card - sixteen: every
// distance between two notes, or between a note and the turn's words,
// is then a whole number of the eight-pixel baseline
// (docs/frontend-method.md §4-33).
export type Rhythm = "line" | "shape";

// A name that leads where there is somewhere to lead, and stands in
// the quieter ink where there is not.
export function named(text: string, href: string | null): Piece {
  return href === null ? { kind: "words", text, ink: "quiet" } : { kind: "link", text, href, face: "text" };
}

// The time a note was written, after the separator every note uses.
export function stamped(text: string): Piece {
  return { kind: "words", text: `· ${text}`, ink: "faint" };
}

const ENDING: Readonly<Record<ReplyEnd, { readonly key: Key; readonly ink: Ink }>> = {
  reply: { key: "talk_reply_ended_reply", ink: "accent" },
  timeout: { key: "talk_reply_ended_timeout", ink: "alert" },
  left: { key: "talk_reply_ended_left", ink: "faint" },
};

// A wait on another room's reply: the room, then the time left before
// the deadline while the wait is open (due once it has passed), or which
// of the three endings closed it, each in its own ink.
export function replyWaitLook(lang: Lang, room: Piece, left: number, ended: ReplyEnded | null | undefined): NoteLook {
  if (ended === undefined || ended === null) {
    const due = left <= 0;
    return {
      role: undefined,
      pieces: [
        { kind: "words", text: say(lang, "talk_reply_wait"), ink: "faint" },
        room,
        {
          kind: "words",
          text: `· ${due ? say(lang, "talk_reply_due") : fill(say(lang, "talk_reply_left"), { left: lasted(left) })}`,
          ink: due ? "alert" : "quiet",
        },
      ],
    };
  }
  const ending = ENDING[ended.by];
  return {
    role: undefined,
    pieces: [
      { kind: "words", text: say(lang, "talk_reply_waited"), ink: "faint" },
      room,
      { kind: "words", text: `· ${say(lang, ending.key)}`, ink: ending.ink },
    ],
  };
}

// A child session handing its work back: the child, then how it ended,
// finished in the accent ink with who verified it, or stopped in the
// alert ink with the reason.
export function handbackLook(lang: Lang, child: Piece, handback: HandbackNote, at: string): NoteLook {
  return {
    role: "note",
    pieces: [
      child,
      { kind: "words", text: say(lang, "talk_handback"), ink: "faint" },
      "finished" in handback
        ? { kind: "words", text: fill(say(lang, "talk_handback_finished"), { by: handback.finished.verified_by }), ink: "accent" }
        : { kind: "words", text: fill(say(lang, "talk_handback_stopped"), { because: handback.stopped.because }), ink: "alert" },
      stamped(at),
    ],
  };
}
