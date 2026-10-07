<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- One tool call, pressed to one line: `kind  subject  time  result`
(refrain §3-4, client/Spec.lean §4-44). A send and a delegation read their
subject and result from the call itself (`call_kind.ts`, client D85).
This file is the line's seat: it reads the call, the clock, the right
side and the line keys, and `call_line.look.svelte` draws what it hands
over (`call_line.ts`).

**The time cell is at least nine characters and grows past that** rather
than spilling over the subject: a long running reading pushes the subject
narrower, and the subject truncates. The line is the trigger of the right
side: pressing it shows the whole call there, through the one door every
opener uses (`inspect/open.svelte.ts`).

**The time cell has two readings and one clock.** While the call runs it
draws tenths from this page's ticker, recomputed from the call's own
moment on every tick; once the result is in the Ledger it draws the
Ledger's milliseconds, and the moment it finished is in the hint as an
ISO instant (docs/frontend-method.md §7D). A span nobody measured draws nothing.

**Keys belong to the line, not to the page.** The line walks by the one
table of line keys (`core/lines.ts`): ↑ and ↓ (and j and k) move to the
line above or below inside the same conversation, Home and End (and gg
and G) to the first and the last; Enter and Space press it; Escape puts
the right side away and keeps the focus here, so the person is still
standing where they were reading. The line that holds the focus, and the
line the right side shows, draw those keys at their end (refrain 3-12's
third layer). -->
<script lang="ts">
  import { fill, say } from "../../core/lang";
  import type { Doing } from "../../core/doing";
  import type { Call, RunId } from "../../wire";
  import { isoInstant } from "../../core/time";
  import { lineWalker } from "../../core/lines";
  import { pressedOf } from "../../core/press";
  import { ui } from "../../ui";
  import { closeRight, openCall, rightItem } from "../inspect/open.svelte";
  import { kindOf, lineOf, outcomeOf } from "./call_kind";
  import { NAMED_AFTER_MS, callTime, runningWords, tookWords, ticker } from "./timing";
  import type { CallLineLook, CallFigure, CallVerdict } from "./call_line";
  import Look from "./call_line.look.svelte";

  interface Props {
    readonly call: Call;
    readonly run: RunId;
    // The run's posture when this line belongs to the turn the run is in
    // now; absent for a turn that is over.
    readonly doing?: Doing | undefined;
  }

  const { call, run, doing }: Props = $props();

  const u = ui();
  const { lang } = u;
  const tick = ticker(u.now);

  const kind = $derived(kindOf(call));
  const line = $derived(lineOf(call));
  const outcome = $derived(outcomeOf(call));
  const time = $derived(callTime(call, call.outcome === "waiting" ? $tick : 0));
  const opened = $derived.by(() => {
    const item = rightItem();
    return item !== null && item.kind === "call" && item.run === run && item.at === call.at;
  });
  // A steer pressed now is heard after this call: the pin stands on the
  // line the run is actually in (refrain §3-5).
  const pinned = $derived(call.outcome === "waiting" && doing?.kind === "calling");
  // Past ten seconds a running line says what it waits for, and only when
  // that is known: a run waiting on the person says so; anything else
  // keeps its silence rather than invent a reason.
  const waitsForYou = $derived(
    call.outcome === "waiting" && doing?.kind === "waiting" && time.kind === "running" && time.ms >= NAMED_AFTER_MS,
  );
  const hint = $derived(
    call.answered === null || call.answered === undefined
      ? fill(say($lang, "talk_call_started"), { at: isoInstant(call.called) })
      : fill(say($lang, "talk_call_finished"), { at: isoInstant(call.answered) }),
  );

  const walk = lineWalker();
  // The keys the line draws at its end, in the order refrain 3-12 names
  // them: up, down, open, close.
  const MOVES_DRAWN = ["line.previous", "line.next", "line.open", "line.close"] as const;

  function linesBeside(from: HTMLElement): HTMLElement[] {
    const root = from.closest("[data-thread]") ?? document;
    return [...root.querySelectorAll<HTMLElement>("[data-call-line]")];
  }

  function onKeydown(event: KeyboardEvent): void {
    const line = event.currentTarget;
    if (!(line instanceof HTMLElement)) return;
    const lines = linesBeside(line);
    const at = lines.indexOf(line);
    const move = walk(pressedOf(event), event.timeStamp);
    const next = (() => {
      switch (move) {
        case "line.next":
          return lines[at + 1];
        case "line.previous":
          return lines[at - 1];
        case "line.first":
          return lines[0];
        case "line.last":
          return lines.at(-1);
        case "line.open":
        case "line.close":
        case null:
          return undefined;
      }
    })();
    if (move === "line.close" && opened) {
      event.preventDefault();
      closeRight();
    } else if (next !== undefined) {
      event.preventDefault();
      next.focus();
    }
  }

  const shownTime = $derived.by((): CallFigure => {
    switch (time.kind) {
      case "landed":
        return { kind: "landed", text: tookWords(time.took, $lang) };
      case "running":
        return { kind: "running", text: runningWords(time.ms) };
      case "starting":
      case "unmeasured":
        return { kind: "none" };
    }
  });
  const verdict = $derived.by((): CallVerdict | null => {
    if (call.outcome === "failed") return { tone: "alert", text: say($lang, "talk_call_failed") };
    if (outcome !== null) return { tone: "quiet", text: say($lang, outcome) };
    if (waitsForYou) return { tone: "alert", text: say($lang, "talk_waiting_you") };
    return null;
  });

  const look = $derived<CallLineLook>({
    kind: kind.kind === "registered" ? say($lang, kind.word) : kind.tool,
    subject: line.kind === "worded" ? fill(say($lang, line.word), line.fills) : line.subject,
    running: call.outcome === "waiting",
    time: shownTime,
    verdict,
    moves: MOVES_DRAWN,
    opened,
    pinned,
    hint,
    wire: {
      type: "button",
      "data-call-line": "",
      "aria-pressed": opened,
      onclick: () => {
        openCall({ run, at: call.at });
      },
      onkeydown: onKeydown,
    },
  });
</script>

<Look {...look} />
