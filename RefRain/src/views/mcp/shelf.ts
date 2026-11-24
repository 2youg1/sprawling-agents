// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { ToolkitLine, ToolkitSlug, ToolkitsAnswer } from "../../wire";
import { readAnswer } from "../../core/answered";
import type { Answered } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { connectToolkit } from "../../core/commands";
import { ui } from "../../ui";
import { get, readable, type Readable } from "../../core/store";

export interface Shelf {
  readonly state: Readable<Answered<ToolkitsAnswer>>;
  readonly answer: () => Answered<ToolkitsAnswer>;
  readonly rows: (state?: Answered<ToolkitsAnswer>) => readonly ToolkitLine[];
  readonly recheck: () => void;
  readonly connect: (slug: ToolkitSlug) => void;
}

export function shelfOf(): Shelf {
  const u = ui();
  const recheck = (): void => {
    u.conn.asking.refresh(QUERIES.toolkits);
  };

  const returned = (): void => {
    if (document.visibilityState === "visible") recheck();
  };

  const state = readable<Answered<ToolkitsAnswer>>({ kind: "asking" }, (set) => {
    const stop = u.conn.asking.ask(QUERIES.toolkits).subscribe((held) => {
      set(readAnswer(held, (each) => ("toolkits" in each ? each.toolkits : undefined)));
    });
    window.addEventListener("focus", returned);
    document.addEventListener("visibilitychange", returned);
    return () => {
      stop();
      window.removeEventListener("focus", returned);
      document.removeEventListener("visibilitychange", returned);
    };
  });

  return {
    state,
    answer: () => get(state),
    rows: (held = get(state)) => {
      return held.kind === "held" && held.value !== "unenrolled" && "shelf" in held.value
        ? held.value.shelf.toolkits
        : [];
    },
    recheck,
    connect: (slug) => {
      if (u.send(connectToolkit(slug))) recheck();
    },
  };
}

export function consentUrl(line: ToolkitLine): string | null {
  const standing = line.standing;
  return standing !== "absent" && "awaiting" in standing ? standing.awaiting.consent_url : null;
}
