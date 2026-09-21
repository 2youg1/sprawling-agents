// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Recording what somebody says, and getting the line of text back.
//
// The browser picks the container: `MediaRecorder` supports a different
// set in each engine, and the one it chose is the one the city is told
// about. The city refuses a container it cannot send on, by name, so a
// guess here would be a guess the person pays for.

import { createSignal } from "solid-js";

import { bearing } from "./socket";

// What a recording attempt ended as. Exhaustive because the composer
// draws a different control for each: a refusal is worth saying, and a
// person who changed their mind is not.
export type Heard =
  | { readonly kind: "text"; readonly text: string }
  | { readonly kind: "refused"; readonly said: string }
  | { readonly kind: "silent" };

// One recording in progress. `stop` resolves once the city has answered.
export interface Recording {
  readonly stop: () => Promise<Heard>;
  readonly cancel: () => void;
}

// Whether this browser can record at all. A page in a tab served over
// plain http to something other than localhost has no microphone, and a
// control that cannot work is a control that should not be drawn.
export function canRecord(): boolean {
  return (
    typeof MediaRecorder !== "undefined" &&
    typeof navigator.mediaDevices !== "undefined"
  );
}

async function ask(origin: string, token: string | null, blob: Blob): Promise<Heard> {
  const answer = await fetch(`${origin}/transcribe`, {
    method: "POST",
    headers: { "content-type": blob.type, ...bearing(token) },
    body: blob,
  });
  const said = await answer.text();
  if (!answer.ok) {
    return { kind: "refused", said };
  }
  return said.trim() === "" ? { kind: "silent" } : { kind: "text", text: said };
}

// Opens the microphone and starts recording. The caller holds the
// handle; nothing here touches the page.
export async function record(origin: string, token: string | null): Promise<Recording | null> {
  const media = await navigator.mediaDevices
    .getUserMedia({ audio: true })
    .catch(() => null);
  if (media === null) {
    return null;
  }
  const recorder = new MediaRecorder(media);
  const parts: Blob[] = [];
  recorder.addEventListener("dataavailable", (event) => {
    if (event.data.size > 0) {
      parts.push(event.data);
    }
  });
  recorder.start();
  const close = () => {
    for (const track of media.getTracks()) {
      track.stop();
    }
  };
  const ended = new Promise<void>((resolve) => {
    recorder.addEventListener("stop", () => {
      resolve();
    });
  });
  return {
    stop: async () => {
      recorder.stop();
      await ended;
      close();
      const blob = new Blob(parts, { type: recorder.mimeType });
      return blob.size === 0 ? { kind: "silent" } : ask(origin, token, blob);
    },
    cancel: () => {
      recorder.stop();
      close();
    },
  };
}

// Dictation as the three states a person can see it in, kept here
// rather than in the box that draws them.
//
// The box owns where words go; this owns whether a recording is
// running, whether the city is still transcribing, and whether the last
// attempt was refused. Splitting it the other way put four signals and
// a two-branch handler inside a view that already had eleven, and the
// state was never about the view: a second control that dictates into
// somewhere else needs the same machine and must not grow a second one.
export interface Dictation {
  readonly taking: () => boolean;
  readonly hearing: () => boolean;
  readonly refused: () => boolean;
  readonly speak: () => void;
}

// `into` is handed the whole new value, not an appendix, so that the
// read of what is already there happens at the moment the city answers
// rather than at the moment the button was pressed: somebody goes on
// typing while a recording is being transcribed.
export function dictation(
  origin: string,
  pairing: string | null,
  into: (heard: string) => void,
): Dictation {
  const [taking, setTaking] = createSignal<Recording | null>(null);
  const [hearing, setHearing] = createSignal(false);
  const [refused, setRefused] = createSignal(false);
  const speak = () => {
    const going = taking();
    if (going === null) {
      setRefused(false);
      void record(origin, pairing).then((started) => {
        setTaking(() => started);
        setRefused(started === null);
      });
      return;
    }
    setTaking(null);
    setHearing(true);
    void going.stop().then((answer: Heard) => {
      setHearing(false);
      setRefused(answer.kind === "refused");
      if (answer.kind === "text") {
        into(answer.text);
      }
    });
  };
  return { taking: () => taking() !== null, hearing, refused, speak };
}
