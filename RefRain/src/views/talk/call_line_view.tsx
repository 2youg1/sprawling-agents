// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId, For, Match, Show, Switch } from "solid-js";
import type { Doing } from "../../core/doing";
import { fill, say } from "../../core/lang";
import { lineWalker } from "../../core/lines";
import { pressedOf } from "../../core/press";
import { storeValue } from "../../core/solid_store";
import { hhmmss } from "../../core/time";
import { ui } from "../../ui";
import type { Call, RunId, Turn } from "../../wire";
import { closeRight, inspectorItems, openCall, rightItem } from "../inspect/open";
import ControlTip from "../parts/control_tip";
import { KbdMarks } from "../parts/kbd.look";
import { chordMarks } from "../parts/kbd_marks";
import { noticeTitle } from "../parts/notice_title";
import { failureOf, kindOf, lineOf, outcomeOf } from "./call_kind";
import type { CallFigure, CallLineLook, CallVerdict } from "./call_line";
import ForkButton from "./fork_button_view";
import type { ForkEntry, ForkPlan } from "./forking";
import SteerPin from "./steer_pin";
import { NAMED_AFTER_MS, callTime, runningWords, ticker, tookWords } from "./timing";

const MOVES = ["line.previous", "line.next", "line.open", "line.close"] as const;

export function CallLineLookView(props: CallLineLook) {
  const id = createUniqueId();
  return (
    <ControlTip id={id} text={props.hint}>
      <button
        type={props.wire.type}
        data-call-line=""
        data-control=""
        aria-describedby={id}
        aria-pressed={props.wire["aria-pressed"] ? "true" : "false"}
        onClick={() => props.wire.onclick()}
        onKeyDown={(event) => props.wire.onkeydown(event)}
        class={[
          "group relative grid h-control w-full grid-cols-[8ch_minmax(8ch,1fr)_minmax(9ch,max-content)_minmax(0,auto)] items-center gap-x-pane rounded-control px-snug text-left text-note narrow:grid-cols-[6ch_minmax(6ch,1fr)_auto_minmax(0,auto)] narrow:gap-x-snug",
          "before:absolute before:inset-y-snug before:left-0 before:w-hair before:rounded-pill",
          props.opened ? "bg-raised text-text before:bg-accent" : "text-text-quiet hover:wash",
        ]}
      >
        <span class="truncate text-text-faint">{props.kind}</span>
        <span class="flex min-w-0 items-center gap-base">
          <span
            class={[
              "min-w-[min(8ch,100%)] flex-1 truncate narrow:min-w-[min(6ch,100%)]",
              props.running && "text-text",
            ]}
          >
            {props.subject}
          </span>
          <Show when={props.failure}>
            {(failure) => (
              <span class="min-w-0 max-w-[24ch] truncate text-alert narrow:max-w-[12ch]">
                {failure()}
              </span>
            )}
          </Show>
        </span>
        <span class="figure flex items-center justify-end gap-tight whitespace-nowrap text-text-faint">
          <Show when={props.running}>
            <span
              class="inline-block size-dot shrink-0 pulse rounded-pill bg-accent"
              aria-hidden="true"
            />
          </Show>
          <Switch>
            <Match when={props.time.kind === "landed" ? props.time : undefined}>
              {(time) => time().text}
            </Match>
            <Match when={props.time.kind === "running" ? props.time : undefined}>
              {(time) => <span class="text-text">{time().text}</span>}
            </Match>
          </Switch>
        </span>
        <span class="flex min-w-0 items-center gap-tight">
          <Show when={props.verdict}>
            {(verdict) => (
              <span
                class={["truncate", verdict().tone === "alert" ? "text-alert" : "text-text-faint"]}
              >
                {verdict().text}
              </span>
            )}
          </Show>
          <span
            data-line-keys=""
            aria-hidden="true"
            class={[
              "items-center gap-hair narrow:hidden",
              props.opened ? "inline-flex" : "hidden group-focus-visible:inline-flex",
            ]}
          >
            <For each={props.moves}>{(move) => <KbdMarks marks={chordMarks({ move })} />}</For>
          </span>
          <Show when={props.pinned}>
            <SteerPin />
          </Show>
        </span>
      </button>
    </ControlTip>
  );
}

export default function CallLine(props: {
  readonly call: Call;
  readonly run: RunId;
  readonly doing?: Doing | undefined;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const tick = storeValue(() => (props.call.outcome === "waiting" ? ticker(u.now) : undefined));
  const selected = storeValue(inspectorItems);
  const time = createMemo(() => callTime(props.call, tick() ?? 0));
  const opened = createMemo(() => {
    const item = rightItem(selected());
    return item?.kind === "call" && item.run === props.run && item.at === props.call.at;
  });
  const walk = lineWalker();
  function onKeyDown(event: KeyboardEvent): void {
    const from = event.currentTarget;
    if (!(from instanceof HTMLElement)) return;
    const root = from.closest("[data-thread]") ?? document;
    const lines = [...root.querySelectorAll<HTMLElement>("[data-call-line]")];
    const at = lines.indexOf(from);
    const move = walk(pressedOf(event), event.timeStamp);
    if (move === "line.close" && opened()) {
      closeRight();
      event.preventDefault();
      return;
    }
    const next =
      move === "line.next"
        ? lines[at + 1]
        : move === "line.previous"
          ? lines[at - 1]
          : move === "line.first"
            ? lines[0]
            : move === "line.last"
              ? lines.at(-1)
              : undefined;
    if (next === undefined) return;
    next.focus();
    event.preventDefault();
  }

  const failedWords = createMemo(() => {
    const failure = failureOf(props.call);
    switch (failure.kind) {
      case "refused":
        return fill(say(lang(), "talk_call_failed_why"), {
          why: noticeTitle(lang(), failure.code, failure.subject),
        });
      case "said":
        return fill(say(lang(), "talk_call_failed_why"), { why: failure.text });
      case "silent":
        return say(lang(), "talk_call_failed");
    }
  });
  const figure = createMemo((): CallFigure => {
    const at = time();
    switch (at.kind) {
      case "landed":
        return { kind: "landed", text: tookWords(at.took, lang()) };
      case "running":
        return { kind: "running", text: runningWords(at.ms) };
      case "starting":
      case "unmeasured":
        return { kind: "none" };
    }
  });
  const verdict = createMemo((): CallVerdict | null => {
    if (props.call.outcome === "failed") return null;
    const outcome = outcomeOf(props.call);
    if (outcome !== null) return { tone: "quiet", text: say(lang(), outcome) };
    const span = time();
    return props.call.outcome === "waiting" &&
      props.doing?.kind === "waiting" &&
      span.kind === "running" &&
      span.ms >= NAMED_AFTER_MS
      ? { tone: "alert", text: say(lang(), "talk_waiting_you") }
      : null;
  });
  const look = createMemo((): CallLineLook => {
    const kind = kindOf(props.call);
    const line = lineOf(props.call);
    const when =
      props.call.answered == null
        ? fill(say(lang(), "talk_call_started"), { at: hhmmss(props.call.called) })
        : fill(say(lang(), "talk_call_finished"), { at: hhmmss(props.call.answered) });
    const failure = props.call.outcome === "failed" ? failedWords() : null;
    return {
      kind: kind.kind === "registered" ? say(lang(), kind.word) : kind.tool,
      subject: line.kind === "worded" ? fill(say(lang(), line.word), line.fills) : line.subject,
      running: props.call.outcome === "waiting",
      time: figure(),
      verdict: verdict(),
      failure,
      moves: MOVES,
      opened: opened(),
      pinned: props.call.outcome === "waiting" && props.doing?.kind === "calling",
      hint: failure === null ? when : `${failure} · ${when}`,
      wire: {
        type: "button",
        "data-call-line": "",
        "aria-pressed": opened(),
        onclick: () => openCall({ run: props.run, at: props.call.at }),
        onkeydown: onKeyDown,
      },
    };
  });
  return <CallLineLookView {...look()} />;
}

export function Calls(props: {
  readonly calls: readonly Call[];
  readonly run: RunId;
  readonly turn: Turn;
  readonly doing?: Doing | undefined;
  readonly onFork?: ((plan: ForkPlan) => void) | undefined;
  readonly onHover: (entry: ForkEntry | null) => void;
}) {
  const lang = storeValue(ui().lang);
  return (
    <ul class="my-tight flex flex-col" aria-label={say(lang(), "talk_calls")}>
      <For each={props.calls} keyed={(call) => call.at}>
        {(call) => (
          <li class="group flex items-center gap-tight [&>span:first-child]:flex-1">
            <CallLine call={call()} run={props.run} doing={props.doing} />
            <Show when={props.onFork}>
              {(fork) => (
                <ForkButton
                  entry={{ kind: "call", turn: props.turn, call: call() }}
                  run={props.run}
                  onFork={fork()}
                  onHover={props.onHover}
                />
              )}
            </Show>
          </li>
        )}
      </For>
    </ul>
  );
}
