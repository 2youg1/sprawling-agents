// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A reply laid out by the city, as a component holds it: the questions
// `core/replying.ts` decides on, asked one at a time through the page's
// `asking`, and the blocks their answers bring (client/Spec.lean §4-26, client D31).
//
// **A settled reply starts from what the streamed one last drew.** The
// thread draws a reply in `talk/saying.svelte` while it streams and in
// `prose.svelte` once its record lands, two components with two states.
// The settled one asks again from the start, so the city reads the whole
// reply at once (D31's two exceptions), and until that answer arrives it
// draws the blocks the streamed one had: otherwise the paragraph a
// person is reading would turn back into raw text for a round trip and
// then into blocks again.

import { untrack } from "svelte";

import { readAnswer } from "../core/answered";
import { UNLAID, answered, question } from "../core/replying";
import type { Laying } from "../core/replying";
import { ui } from "../ui";
import type { Block, ReplyState } from "../wire";

// What one reply has drawn: its blocks, and how much of its text, in
// UTF-16 code units, they cover. The rest is drawn as it is.
export interface Drawn {
  readonly blocks: readonly Block[];
  readonly reached: number;
}

// What a streamed reply last drew, under the stretch of text it covers.
interface Streamed {
  readonly covered: string;
  readonly blocks: readonly Block[];
}

// A few replies stream at once at most - one per run on screen - and a
// settled reply takes over within a round trip of its stream ending.
const STREAMED_KEPT = 4;
let streamed: readonly Streamed[] = [];

function handOver(covered: string, blocks: readonly Block[]): void {
  const others = streamed.filter((each) => !covered.startsWith(each.covered));
  streamed = [...others.slice(-(STREAMED_KEPT - 1)), { covered, blocks }];
}

function takenOver(text: string): Laying | null {
  const longest = streamed
    .filter((each) => each.covered !== "" && text.startsWith(each.covered))
    .reduce<Streamed | null>((best, each) => (best === null || each.covered.length > best.covered.length ? each : best), null);
  return longest === null ? null : { ...UNLAID, blocks: longest.blocks, reached: longest.covered.length };
}

// Called while a component initialises, with the text it draws read
// through a getter so the questions follow it as it grows.
export function laidReply(said: () => string, state: ReplyState): Drawn {
  const asking = ui().conn.asking;
  const before = state === "settled" ? takenOver(said()) : null;
  let laying = $state<Laying>(UNLAID);
  // The stretch of text the blocks were read from.
  let covered = "";
  // The text of the one question in flight.
  let asked = $state<string | null>(null);

  $effect(() => {
    const text = said();
    untrack(() => {
      // A text that no longer begins with what was laid out is another
      // reply in the same place: start again, and drop the question
      // still out about the old one.
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
