// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, For, onSettled, Show } from "solid-js";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import type { RunBelief } from "../../core/belief";
import { runText } from "../../core/belief/runs";
import { keymap } from "../../core/keys";
import { fill, say } from "../../core/lang";
import { drawsCalls } from "../../core/results";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { clock, count } from "../../core/time";
import { ui } from "../../ui";
import type { Query, Seq } from "../../wire";
import Unanswered from "../parts/unanswered.solid";
import { phaseOf } from "../runs/lineage";
import { turnsAround } from "./around";
import { heard, landed, lost } from "./arrivals";
import { callWord } from "./calls";
import Failed from "./failed_view";
import { planFork, roundsThrough, type ForkEntry, type ForkPlan } from "./forking";
import { firstHeadSaid } from "./frozen";
import { called, dispatcherOf } from "./naming";
import Person from "./person_view";
import Reasoning from "./reasoning_view";
import Rule from "./rule_view";
import { restriction, sandboxQuery, sandboxSaid } from "./sandbox";
import Saying from "./saying_view";
import type { FrozenFacts } from "./seat";
import { cutOff, silentRun, type Phase } from "./silence";
import { sourcesOf } from "./source_context";
import SteerPin from "./steer_pin";
import TurnView from "./turn";

export interface ThreadProps {
  readonly through?: Seq | null | undefined;
  readonly run: RunBelief;
  readonly stream?: RunBelief | undefined;
  readonly onFork?: ((plan: ForkPlan) => void) | undefined;
  readonly onRetry?: ((task: string) => void) | undefined;
  readonly facts?: FrozenFacts | undefined;
  readonly around?: Seq | null | undefined;
}

export default function Thread(props: ThreadProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const held = storeValue(u.prefs.held);
  let active: ForkEntry | null = null;
  const hoverFork = (entry: ForkEntry | null): void => {
    active = entry;
  };
  const question = createMemo<Query>(() => ({ rounds: { run: props.run.run } }));
  const rounds = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(rounds(), (held) => ("rounds" in held ? held.rounds : undefined)),
  );
  const answer = createMemo(() => {
    const found = read();
    return found.kind === "held" ? roundsThrough(found.value, props.through ?? null) : undefined;
  });
  const turns = createMemo(() => answer()?.turns ?? []);
  const drawn = createMemo(() => turnsAround(turns(), props.around ?? null));
  const shown = createMemo(() => turns().slice(drawn().from, drawn().to));
  const task = createMemo(() => answer()?.opening?.task ?? props.run.task ?? "");
  const sources = createMemo(() => sourcesOf(task(), turns()));
  const spoken = storeValue(() => runText(props.stream ?? props.run, "saying"));
  const thoughts = storeValue(() => runText(props.stream ?? props.run, "thinking"));
  const taskEntry = createMemo<ForkEntry | null>(() => {
    const first = turns().at(0);
    return first === undefined ? null : { kind: "message", turn: first, text: task() };
  });
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const ceiling = createMemo(() => {
    const answer = endpoints();
    return answer === undefined || !("endpoints" in answer)
      ? null
      : (answer.endpoints.chosen.find((each) => each.tag === "main")?.max_output_tokens ?? null);
  });
  const frozen = createMemo(() => props.run.doing.kind === "frozen");
  const phase = createMemo<Phase>(() => (frozen() ? "frozen" : "live"));
  const word = createMemo(() => {
    const doing = props.run.doing;
    return answer()?.closing?.completion ?? (doing.kind === "frozen" ? doing.completion : null);
  });
  const completion = createMemo((): string => {
    const named = word();
    switch (named) {
      case "done":
        return say(lang(), "outcome_done");
      case "cancelled":
        return say(lang(), "outcome_cancelled");
      case "limit":
        return say(lang(), "outcome_limit");
      case null:
        return say(lang(), "outcome_frozen");
      default:
        return named;
    }
  });
  const streaming = createMemo(() => props.through == null && !frozen() && spoken().length > 0);
  const closedAt = createMemo(() => answer()?.closing?.at ?? null);
  const emptyRun = createMemo(() => read().kind === "held" && silentRun(turns(), phase()));
  const failure = createMemo(() => answer()?.closing?.failure ?? null);
  const stopped = createMemo(() => word() === "cancelled");
  const wayOut = createMemo(() =>
    failure() !== null ? undefined : stopped() ? ("absent" as const) : ("offered" as const),
  );
  const why = createMemo(() => cutOff(turns().at(-1)?.stopped));
  const whole = createMemo(() => drawsCalls(held().showing));
  const lineSaysIt = createMemo(
    () =>
      whole() &&
      (props.run.doing.kind === "calling" || props.run.doing.kind === "waiting") &&
      (turns()
        .at(-1)
        ?.calls.some((call) => call.outcome === "waiting") ??
        false),
  );
  const who = createMemo(() =>
    props.run.addr === null
      ? say(lang(), "talk_resident")
      : called(props.run.addr, answer()?.opening?.names?.mayor, lang()),
  );
  const from = createMemo((): { readonly label: string; readonly href: string | undefined } => {
    const opening = answer()?.opening;
    const by = dispatcherOf(opening?.dispatched_by);
    const parent = opening?.parent ?? null;
    switch (by?.kind) {
      case undefined:
      case "person":
        return { label: say(lang(), "talk_you"), href: undefined };
      case "city":
        return { label: say(lang(), "talk_dispatched_city"), href: undefined };
      case "resident":
        return {
          label: fill(say(lang(), "talk_dispatched_by"), {
            who: called(by.address, opening?.names?.mayor, lang()),
          }),
          href: toFragment(
            parent === null ? { kind: "talk", address: by.address } : { kind: "run", run: parent },
          ),
        };
    }
  });
  const building = storeValue(() =>
    (props.facts ?? "head") === "head" && props.run.addr !== null
      ? u.conn.asking.ask(sandboxQuery(props.run.addr))
      : null,
  );
  const firstStated = createMemo(() => {
    const bounded = restriction(building());
    const facts = [
      turns().at(0)?.model ?? "",
      firstHeadSaid(answer()?.opening, lang()),
      bounded === null ? "" : sandboxSaid(bounded, lang()),
    ]
      .filter((part) => part !== "")
      .join(" · ");
    return facts === "" ? null : facts;
  });
  const stated = createMemo(() =>
    turns().map((turn, at) => {
      const model = turn.model ?? null;
      if (at === 0) return (props.facts ?? "head") === "head" ? firstStated() : null;
      return model !== null && model !== (turns()[at - 1]?.model ?? null) ? model : null;
    }),
  );

  let streamedInto: Seq | null = null;
  let wasStreaming = false;
  createEffect(
    () => spoken().length,
    (length) => {
      heard(props.run.run, length, u.now());
    },
  );
  createEffect(
    () => ({ now: streaming(), newest: turns().at(-1) }),
    ({ now, newest }) => {
      if (now && newest !== undefined && (newest.said ?? "") === "") streamedInto = newest.opened;
      if (!now && wasStreaming) {
        if (streamedInto === null) lost(props.run.run);
        else landed(props.run.run, streamedInto);
        streamedInto = null;
      }
      wasStreaming = now;
    },
  );

  const posture = createMemo((): string => {
    const doing = props.run.doing;
    switch (doing.kind) {
      case "thinking":
        return fill(say(lang(), "talk_thinking"), { who: who() });
      case "calling":
        return doing.tool === null
          ? say(lang(), "run_doing_calling")
          : callWord(doing.tool, doing.subject);
      case "waiting":
        return say(lang(), "talk_waiting_you");
      case "awaiting_reply":
        return say(lang(), "run_doing_awaiting_reply");
      case "unknown":
        return say(lang(), "city_at_work");
      case "frozen":
        return "";
    }
  });

  function forkKey(event: KeyboardEvent): void {
    const action = keymap().acting({
      key: event.key,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
      target: inField(event.target) ? "field" : "page",
    });
    if (action !== "fork.here" || active === null || props.onFork === undefined) return;
    event.preventDefault();
    props.onFork(planFork(props.run.run, active));
  }
  onSettled(() => {
    window.addEventListener("keydown", forkKey);
    return () => {
      window.removeEventListener("keydown", forkKey);
    };
  });

  const unavailable = createMemo(() => {
    const heading = read();
    return heading.kind === "unavailable" ? heading : null;
  });
  const failureWord = createMemo(() => {
    if (stopped()) return say(lang(), "talk_failed_cancelled");
    const limit = ceiling();
    return limit === null
      ? say(lang(), "talk_said_nothing")
      : fill(say(lang(), "talk_said_nothing_capped"), { n: count(limit) });
  });
  const closingWord = createMemo(() => {
    const at = closedAt();
    return at === null ? completion() : `${completion()} · ${clock(lang(), at)}`;
  });

  return (
    <section aria-label={props.run.run} class={frozen() ? "settled" : undefined}>
      <Show when={task() !== ""}>
        <Person
          text={task()}
          label={from().label}
          labelHref={from().href}
          at={props.run.started ?? undefined}
          entry={taskEntry()}
          run={props.run.run}
          onFork={props.onFork}
          onHover={hoverFork}
        />
      </Show>
      <Show when={unavailable()}>
        {(found) => <Unanswered query={found().query} asked={question()} />}
      </Show>
      <For each={shown()} keyed={(turn) => turn.opened}>
        {(turn, visible) => {
          const at = () => visible() + drawn().from;
          return (
            <TurnView
              turn={turn()}
              sources={sources()}
              run={props.run.run}
              who={who()}
              model={stated()[at()] ?? null}
              live={phase()}
              doing={at() === turns().length - 1 && !frozen() ? props.run.doing : undefined}
              showEmpty={!emptyRun()}
              ceiling={ceiling()}
              whole={whole()}
              onFork={props.onFork}
              onHover={hoverFork}
            />
          );
        }}
      </For>
      <Show when={drawn().cut}>
        <p class="my-snug text-note text-text-faint">
          <a
            href={toFragment({ kind: "run", run: props.run.run })}
            class="underline decoration-from-font underline-offset-2 transition-colors ease-leave hover:text-text hover:ease-arrive"
          >
            {say(lang(), "letter_whole_talk")}
          </a>
        </p>
      </Show>
      <Show when={props.through == null && !frozen() && thoughts().length > 0 && whole()}>
        <div class="my-tight">
          <Reasoning text={thoughts()} open />
        </div>
      </Show>
      <Show when={streaming()}>
        <Saying text={spoken()} who={who()} />
        <SteerPin />
      </Show>
      <Show when={emptyRun()}>
        <Failed
          what={failureWord()}
          error={failure() ?? undefined}
          settings={wayOut()}
          onRetry={
            props.onRetry === undefined
              ? undefined
              : () => {
                  props.onRetry?.(task());
                }
          }
        />
        <Show when={why()}>
          {(stopped) => (
            <div class="text-note text-text-faint">
              {fill(say(lang(), "talk_cut_off"), { why: stopped() })}
            </div>
          )}
        </Show>
      </Show>
      <Show when={!frozen() && !streaming() && !lineSaysIt()}>
        <div class="my-snug flex items-center gap-snug text-note text-text-faint">
          <span class="thinking-mark" aria-hidden="true">
            <i />
            <i />
            <i />
          </span>
          <span>{posture()}</span>
          <Show when={props.run.doing.kind === "thinking"}>
            <SteerPin />
          </Show>
        </div>
      </Show>
      <Show when={frozen()}>
        <div class="my-wide">
          <Rule
            parts={[
              {
                kind: "link",
                text: closingWord(),
                href: toFragment({ kind: "run", run: props.run.run }),
              },
            ]}
            mark="none"
            wire={{ role: "status", "data-wear": phaseOf(props.run.doing) }}
          />
        </div>
      </Show>
    </section>
  );
}

function inField(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}
