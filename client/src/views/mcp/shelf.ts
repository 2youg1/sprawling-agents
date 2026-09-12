// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The broker's shelf, and the one press that connects a row of it.
//
// The directory is the broker's and is asked for, never held here: it
// changes weekly, and a copy compiled into this client would be a
// second authority that is wrong by the time anybody reads it.
//
// **Nothing here polls.** The shelf is re-asked when a person opens the
// page, when they press connect, and when they come back to this window
// from the consent page they were sent to. That last one is why the
// flow needs no timer at all: the person leaves to give consent and
// returns, and returning is the event.

import { createMemo, onCleanup } from "solid-js";

import type { ToolkitLine, ToolkitSlug, ToolkitsAnswer } from "../../wire";
import { connectToolkit } from "../../core/commands";
import { useCommand, useUi } from "../../ui";

export interface Shelf {
  // What the city last answered. `undefined` until the first answer
  // lands, which the page draws as waiting rather than as empty.
  readonly answer: () => ToolkitsAnswer | undefined;
  readonly rows: () => readonly ToolkitLine[];
  // Ask the broker again. Explicit rather than folded into a record's
  // arrival: this read costs a round trip to somebody else, and a
  // person is waiting in front of it.
  readonly recheck: () => void;
  readonly connect: (slug: ToolkitSlug) => void;
}

export function shelfOf(): Shelf {
  const ui = useUi();
  const command = useCommand();
  const asked = createMemo(() => ui.conn.asking.ask("toolkits"));
  const answer = (): ToolkitsAnswer | undefined => {
    const held = asked()();
    return held !== undefined && "toolkits" in held ? held.toolkits : undefined;
  };
  const recheck = () => {
    ui.conn.asking.refresh("toolkits");
  };

  // Coming back to this window is the one signal that the consent page
  // is finished with, and it costs nothing when it is not: the same
  // read the page already makes, made again.
  const returned = () => {
    if (document.visibilityState === "visible") recheck();
  };
  window.addEventListener("focus", returned);
  document.addEventListener("visibilitychange", returned);
  onCleanup(() => {
    window.removeEventListener("focus", returned);
    document.removeEventListener("visibilitychange", returned);
  });

  return {
    answer,
    rows: () => {
      const held = answer();
      return held !== undefined && held !== "unenrolled" && "shelf" in held
        ? held.shelf.toolkits
        : [];
    },
    recheck,
    connect: (slug) => {
      // The city opens the session with the broker and records that it
      // was asked for; where the row now stands, and the page to send
      // the person to, come back on the next read.
      if (command(connectToolkit(slug))) recheck();
    },
  };
}

/// The consent page a row is waiting on, or null when it waits on
/// nothing. Kept as a value the page can offer twice: a blocked popup
/// and a closed tab both leave somebody who needs this link again.
export function consentUrl(line: ToolkitLine): string | null {
  const standing = line.standing;
  return standing !== "absent" && "awaiting" in standing ? standing.awaiting.consent_url : null;
}
