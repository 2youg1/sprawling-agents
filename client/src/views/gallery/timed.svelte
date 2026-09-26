<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The run page's summary bar over its time lens, twice: a run of
  // forty turns that waited once for the person and is still calling a
  // tool, and a finished run of ten thousand tool calls, which is the
  // one that shows the lane holding one box per column and the call
  // list holding only the rows in view.
  //
  // **The turns are derived, not typed out.** Their lengths come from a
  // fixed generator, so every screenshot draws the same river.

  import { Address, Seq, TimeMs, Tokens, UsdMicros, type Call, type Note, type Turn } from "../../wire";

  const START = 1_790_000_000_000;

  function seeded(): () => number {
    let state = 7;
    return () => {
      state = (state * 1_103_515_245 + 12_345) >>> 0;
      return (state >>> 8) / 16_777_216;
    };
  }

  function turnsOf(count: number, callsEach: number): readonly Turn[] {
    const next = seeded();
    let at = START;
    return Array.from({ length: count }, (_, n) => {
      const span = n === 17 ? 140_000 : 4_000 + Math.floor(next() * 30_000);
      const calls: Call[] = Array.from({ length: n % 4 === 3 ? 0 : callsEach }, (_, c) => ({
        tool: ["read", "search", "edit", "exec"][c % 4] ?? "read",
        subject: `crates/city/src/part_${String((n * 7 + c) % 97)}.rs`,
        arguments: null,
        outcome: (n + c) % 23 === 0 ? "failed" : "answered",
        at: Seq.make(n * 100 + c),
        output: null,
        called: TimeMs.make(at + 1_000 + c * 600),
        answered: TimeMs.make(at + 1_400 + c * 600),
      }));
      const notes: Note[] = n === 17 ? [{ waiting: { at: Seq.make(n * 100 + 99) } }] : [];
      const turn: Turn = {
        calls,
        notes,
        number: n + 1,
        opened: Seq.make(n * 100),
        t: TimeMs.make(at),
        used: { input: Tokens.make(12_000 + n * 400), output: Tokens.make(900 + n * 20), cached: Tokens.make(n * 300) },
        spent: UsdMicros.make(40_000 + n * 1_000),
      };
      at += span;
      return turn;
    });
  }

  export const LIVE = turnsOf(40, 3);
  export const LONG = turnsOf(2_500, 4);
  export const ROOM = Address.make("shop/checkout");
</script>

<script lang="ts">
  import Head from "../run/head.svelte";
  import River from "../run/river.svelte";
  import Case from "./case.svelte";

  function endOf(turns: readonly Turn[]): number {
    return (turns.at(-1)?.t ?? START) + 20_000;
  }
</script>

<Case label="run · the summary bar over the time lens, live and calling a tool">
  <Head
    turns={LIVE}
    doing={{ kind: "calling", tool: "exec", subject: "just check" }}
    closing={null}
    room={ROOM}
    from={START}
    to={endOf(LIVE)}
  />
  <River turns={LIVE} from={START} to={endOf(LIVE)} tail="tool" />
</Case>

<Case label="run · ten thousand tool calls, drawn per column and per view">
  <Head
    turns={LONG}
    doing={{ kind: "frozen", completion: "done" }}
    closing={{ at: TimeMs.make(endOf(LONG)), completion: "done" }}
    room={ROOM}
    from={START}
    to={endOf(LONG)}
  />
  <River turns={LONG} from={START} to={endOf(LONG)} tail={null} />
</Case>
