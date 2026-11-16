// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { untrack } from "svelte";

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

export function laidReply(said: () => string, state: ReplyState): Drawn {
  const asking = ui().conn.asking;
  const before = state === "settled" ? takenOver(said()) : null;
  let laying = $state<Laying>(UNLAID);

  let covered = "";

  let asked = $state<string | null>(null);

  $effect(() => {
    const text = said();
    untrack(() => {
      if (!text.startsWith(covered)) {
        laying = UNLAID;
        covered = "";
        asked = null;
      }
      asked ??= question(text, laying, state);
    });
  });

  $effect(() => {
    const text = asked;
    if (text === null) return;
    return asking.ask({ reply: { text, state } }).subscribe((answer) => {
      untrack(() => {
        const read = readAnswer(answer, (held) => ("reply" in held ? held.reply : undefined));
        if (read.kind === "asking") return;
        laying = answered(laying, text, read, state);
        covered = said().slice(0, laying.reached);
        asked = question(said(), laying, state);
        if (state === "streaming") handOver(covered, laying.blocks);
      });
    });
  });

  const shown = $derived(before !== null && laying === UNLAID ? before : laying);
  return {
    get blocks() {
      return shown.blocks;
    },
    get reached() {
      return shown.reached;
    },
  };
}
