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

export interface Shelf {
  readonly answer: () => Answered<ToolkitsAnswer>;
  readonly rows: () => readonly ToolkitLine[];
  readonly recheck: () => void;
  readonly connect: (slug: ToolkitSlug) => void;
}

export function shelfOf(): Shelf {
  const u = ui();
  let answer = $state<Answered<ToolkitsAnswer>>({ kind: "asking" });

  $effect(() =>
    u.conn.asking.ask(QUERIES.toolkits).subscribe((held) => {
      answer = readAnswer(held, (each) => ("toolkits" in each ? each.toolkits : undefined));
    }),
  );

  const recheck = (): void => {
    u.conn.asking.refresh(QUERIES.toolkits);
  };

  const returned = (): void => {
    if (document.visibilityState === "visible") recheck();
  };

  $effect(() => {
    window.addEventListener("focus", returned);
    document.addEventListener("visibilitychange", returned);
    return () => {
      window.removeEventListener("focus", returned);
      document.removeEventListener("visibilitychange", returned);
    };
  });

  return {
    answer: () => answer,
    rows: () => {
      const held = answer;
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
