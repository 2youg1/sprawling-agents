// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { readonly, writable } from "../core/store";

import { readAnswer } from "../core/answered";
import { UNLAID, answered, question } from "../core/replying";
import type { Laying } from "../core/replying";
import { ui } from "../ui";
import type { Block, ReplyState } from "../wire";

export interface Drawn {
  readonly blocks: readonly Block[];
  readonly reached: number;
}

interface Streamed {
  readonly covered: string;
  readonly blocks: readonly Block[];
}

const STREAMED_KEPT = 4;
let streamed: readonly Streamed[] = [];

function handOver(covered: string, blocks: readonly Block[]): void {
  const others = streamed.filter((each) => !covered.startsWith(each.covered));
  streamed = [...others.slice(-(STREAMED_KEPT - 1)), { covered, blocks }];
}

function takenOver(text: string): Laying | null {
  const longest = streamed
    .filter((each) => each.covered !== "" && text.startsWith(each.covered))
    .reduce<Streamed | null>(
      (best, each) => (best === null || each.covered.length > best.covered.length ? each : best),
      null,
    );
  return longest === null
    ? null
    : { ...UNLAID, blocks: longest.blocks, reached: longest.covered.length };
}

export function laidReply(said: () => string, state: ReplyState) {
  const asking = ui().conn.asking;
  const before = state === "settled" ? takenOver(said()) : null;
  const view = writable<Drawn>(before ?? UNLAID);
  let laying: Laying = UNLAID;
  let covered = "";
  let asked: string | null = null;
  let stop: (() => void) | undefined;
  let open = true;

  function seal(): void {
    asked = null;
    stop?.();
    stop = undefined;
  }

  function advance(text: string): void {
    if (!open) return;
    if (!text.startsWith(covered)) {
      seal();
      laying = UNLAID;
      covered = "";
      view.set(before ?? UNLAID);
    }
    if (asked !== null) return;
    const next = question(text, laying, state);
    if (next === null) return;
    asked = next;
    const release = asking.ask({ reply: { text: next, state } }).subscribe((answer) => {
      if (asked !== next) return;
      const read = readAnswer(answer, (held) => ("reply" in held ? held.reply : undefined));
      if (read.kind === "asking") return;
      const updated = answered(laying, next, read, state);
      if (updated === laying) return;
      seal();
      laying = updated;
      covered = said().slice(0, laying.reached);
      if (state === "streaming") handOver(covered, laying.blocks);
      view.set(laying);
      queueMicrotask(() => {
        advance(said());
      });
    });
    if (asked === next) stop = release;
    else release();
  }

  return {
    state: readonly(view),
    update: advance,
    close() {
      open = false;
      seal();
    },
  };
}
