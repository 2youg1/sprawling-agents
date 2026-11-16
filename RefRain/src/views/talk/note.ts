// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { lasted } from "../../core/time";
import type { HandbackNote, ReplyEnd, ReplyEnded } from "../../wire";

export type Ink = "faint" | "quiet" | "accent" | "alert";

export type Face = "text" | "mono";

export type Piece =
  | { readonly kind: "words"; readonly text: string; readonly ink: Ink }
  | { readonly kind: "link"; readonly text: string; readonly href: string; readonly face: Face };

export interface NoteLook {
  readonly role: "note" | "status" | undefined;
  readonly pieces: readonly Piece[];
}

export interface LetterLook {
  readonly label: string;
  readonly head: readonly Piece[];
  readonly said: string;
}

export type Rhythm = "line" | "shape";

export function named(text: string, href: string | null): Piece {
  return href === null
    ? { kind: "words", text, ink: "quiet" }
    : { kind: "link", text, href, face: "text" };
}

export function stamped(text: string): Piece {
  return { kind: "words", text: `· ${text}`, ink: "faint" };
}

const ENDING: Readonly<Record<ReplyEnd, { readonly key: Key; readonly ink: Ink }>> = {
  reply: { key: "talk_reply_ended_reply", ink: "accent" },
  timeout: { key: "talk_reply_ended_timeout", ink: "alert" },
  left: { key: "talk_reply_ended_left", ink: "faint" },
};

export function replyWaitLook(
  lang: Lang,
  room: Piece,
  left: number,
  ended: ReplyEnded | null | undefined,
): NoteLook {
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

export function handbackLook(
  lang: Lang,
  child: Piece,
  handback: HandbackNote,
  at: string,
): NoteLook {
  return {
    role: "note",
    pieces: [
      child,
      { kind: "words", text: say(lang, "talk_handback"), ink: "faint" },
      "finished" in handback
        ? {
            kind: "words",
            text: fill(say(lang, "talk_handback_finished"), { by: handback.finished.verified_by }),
            ink: "accent",
          }
        : {
            kind: "words",
            text: fill(say(lang, "talk_handback_stopped"), { because: handback.stopped.because }),
            ink: "alert",
          },
      stamped(at),
    ],
  };
}
