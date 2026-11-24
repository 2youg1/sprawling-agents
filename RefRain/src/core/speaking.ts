// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { derived, get, writable } from "./store";
import type { Readable } from "./store";

import { bearing } from "./socket";

export type Heard =
  | { readonly kind: "text"; readonly text: string }
  | { readonly kind: "refused"; readonly said: string }
  | { readonly kind: "silent" };

export interface Recording {
  readonly stop: () => Promise<Heard>;
  readonly cancel: () => void;
}

export function canRecord(): boolean {
  return typeof MediaRecorder !== "undefined" && typeof navigator.mediaDevices !== "undefined";
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

export async function record(origin: string, token: string | null): Promise<Recording | null> {
  const media = await navigator.mediaDevices.getUserMedia({ audio: true }).catch(() => null);
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

export interface Dictation {
  readonly taking: Readable<boolean>;
  readonly hearing: Readable<boolean>;
  readonly refused: Readable<boolean>;
  readonly speak: () => void;
}

export function dictation(
  origin: string,
  credential: () => string | null,
  into: (heard: string) => void,
): Dictation {
  const taking = writable<Recording | null>(null);
  const hearing = writable(false);
  const refused = writable(false);
  const speak = () => {
    const going = get(taking);
    if (going === null) {
      refused.set(false);
      void record(origin, credential()).then((started) => {
        taking.set(started);
        refused.set(started === null);
      });
      return;
    }
    taking.set(null);
    hearing.set(true);
    void going.stop().then((answer: Heard) => {
      hearing.set(false);
      refused.set(answer.kind === "refused");
      if (answer.kind === "text") {
        into(answer.text);
      }
    });
  };
  return {
    taking: derived(taking, (held) => held !== null),
    hearing,
    refused,
    speak,
  };
}
