<script lang="ts">
  import { fill, say } from "../../core/lang";
  import type { Doing } from "../../core/doing";
  import type { Call, RunId } from "../../wire";
  import { hhmmss } from "../../core/time";
  import { lineWalker } from "../../core/lines";
  import { pressedOf } from "../../core/press";
  import { ui } from "../../ui";
  import { closeRight, openCall, rightItem } from "../inspect/open.svelte";
  import { failureOf, kindOf, lineOf, outcomeOf } from "./call_kind";
  import { noticeTitle } from "../parts/notice_title";
  import { NAMED_AFTER_MS, callTime, runningWords, tookWords, ticker } from "./timing";
  import type { CallLineLook, CallFigure, CallVerdict } from "./call_line";
  import Look from "./call_line.look.svelte";

  interface Props {
    readonly call: Call;
    readonly run: RunId;

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

  const pinned = $derived(call.outcome === "waiting" && doing?.kind === "calling");

  const waitsForYou = $derived(
    call.outcome === "waiting" &&
      doing?.kind === "waiting" &&
      time.kind === "running" &&
      time.ms >= NAMED_AFTER_MS,
  );

  const failedWords = $derived.by((): string => {
    const failure = failureOf(call);
    switch (failure.kind) {
      case "refused":
        return fill(say($lang, "talk_call_failed_why"), {
          why: noticeTitle($lang, failure.code, failure.subject),
        });
      case "said":
        return fill(say($lang, "talk_call_failed_why"), { why: failure.text });
      case "silent":
        return say($lang, "talk_call_failed");
    }
  });
  const when = $derived(
    call.answered === null || call.answered === undefined
      ? fill(say($lang, "talk_call_started"), { at: hhmmss(call.called) })
      : fill(say($lang, "talk_call_finished"), { at: hhmmss(call.answered) }),
  );
  const hint = $derived(call.outcome === "failed" ? `${failedWords} · ${when}` : when);

  const walk = lineWalker();

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
    if (call.outcome === "failed") return null;
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
    failure: call.outcome === "failed" ? failedWords : null,
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
