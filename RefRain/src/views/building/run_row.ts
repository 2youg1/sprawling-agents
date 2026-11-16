// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one run of a room is given (client D95: `directory.svelte` is the
// seat, `run_row.look.svelte` draws the row). The row is a link to the
// run's page; every line on it is already in the person's language.

export interface RunRowWire {
  readonly href: string;
}

export interface RunRowLook {
  readonly task: string;
  // Whether the run still works; a frozen run's dot is the mark ink.
  readonly live: boolean;
  readonly posture: string;
  // What it cost and when it started, where the page knows.
  readonly spent: string | undefined;
  readonly started: string | undefined;
  readonly wire: RunRowWire;
}
