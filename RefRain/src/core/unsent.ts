// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { writable } from "./store";
import type { Readable } from "./store";

import type { Command } from "../wire";

export interface Unsent {
  readonly count: Readable<number>;
  hold(command: Command): void;
  release(send: (command: Command) => boolean): void;
}

export function isSpeech(command: Command): boolean {
  return "dispatch" in command || "steer" in command;
}

export function createUnsent(): Unsent {
  const waiting: Command[] = [];
  const count = writable(0);
  return {
    count,
    hold(command) {
      waiting.push(command);
      count.set(waiting.length);
    },
    release(send) {
      const sent = waiting.findIndex((command) => !send(command));
      waiting.splice(0, sent === -1 ? waiting.length : sent);
      count.set(waiting.length);
    },
  };
}
