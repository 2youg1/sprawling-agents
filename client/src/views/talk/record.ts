// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the microphone's look is handed: which of its three states holds,
// its name for that state, the refusal to say beside it, and its wiring
// as one bag the look spreads on its button. One press records, a second
// stops, and a press while the city is still transcribing does nothing
// (client/Spec.lean §4-16).

import type { HTMLButtonAttributes } from "svelte/elements";

import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";

// Recording the person, waiting for the city's transcription, or neither.
export type Hearing = "taking" | "hearing" | "ready";

export interface RecordLook {
  readonly hearing: Hearing;
  readonly label: string;
  // The city refused the last recording; said beside the button.
  readonly refused: string | undefined;
  readonly wire: RecordWire;
}

export type RecordWire = Pick<HTMLButtonAttributes, "type" | "aria-disabled" | "onclick">;

export interface Heard {
  readonly taking: boolean;
  readonly hearing: boolean;
  readonly refused: boolean;
}

// Transcribing outranks recording: the recording has already stopped
// when the city starts to transcribe it.
export function hearingOf(heard: Heard): Hearing {
  if (heard.hearing) return "hearing";
  return heard.taking ? "taking" : "ready";
}

export function lookOf(heard: Heard, lang: Lang, press: () => void): RecordLook {
  const hearing = hearingOf(heard);
  return {
    hearing,
    label: say(lang, LABEL[hearing]),
    refused: heard.refused ? say(lang, "link_refused") : undefined,
    wire: {
      type: "button",
      "aria-disabled": hearing === "hearing" ? "true" : undefined,
      // The seat's `press` refuses while the city transcribes, for this
      // button and for the palette's "speak" alike.
      onclick: press,
    },
  };
}

const LABEL = {
  taking: "talk_recording",
  hearing: "talk_hearing",
  ready: "talk_record",
} as const;
