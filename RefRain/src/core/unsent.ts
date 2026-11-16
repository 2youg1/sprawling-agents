// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a person said while the link was down, kept in order until the
// city greets the page again. Only words wait: a stop, a release or an
// approval pressed now and carried out a minute later may no longer be
// what the person wanted, so those are refused at the press instead.

import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

import type { Command } from "../wire";

export interface Unsent {
  // How many are waiting; the disconnect banner says this number.
  readonly count: Readable<number>;
  hold(command: Command): void;
  // Sends in order and stops at the first that does not go, keeping it
  // and everything after it for the next welcome.
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
