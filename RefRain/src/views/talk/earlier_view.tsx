// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, For, Match, Show, Switch } from "solid-js";
import type { RunBelief } from "../../core/belief";
import { fill, say } from "../../core/lang";
import { outcomeOf } from "../../core/results";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { clock, hhmm } from "../../core/time";
import { ui } from "../../ui";
import type { Seq } from "../../wire";
import { lastCheckpointIn } from "../checkpoints";
import Glyph from "../parts/glyph.solid";
import { OUTCOME_GLYPH, OUTCOME_INK } from "../shared/outcome";
import { earlierDrawn } from "./earlier";
import { foldWire } from "./fold";
import { motherName, originalOf, roundsThrough, type Boundary, type ForkPlan } from "./forking";
import Grounded from "./grounded";
import Produced from "./produced.solid";
import type { RuleLook, RulePart } from "./rule";
import Rule, { AsideLink, Fold } from "./rule_view";
import { sourcesOf, withoutSources } from "./source_context";
import Thread from "./thread";

interface EarlierProps {
  readonly session?: Seq | null | undefined;
  readonly earlier: readonly RunBelief[];
  readonly boundary: Boundary | null;
}

function useFold(props: EarlierProps, shown: () => number) {
  const [pressed, setPressed] = createSignal<{
    readonly session: Seq | null;
    readonly open: boolean;
  }>();
  const drawn = createMemo(() =>
    earlierDrawn(shown(), props.earlier.length, props.boundary?.kind === "forked"),
  );
  const open = createMemo(() => {
    const current = pressed();
    return current?.session === (props.session ?? null) ? current.open : drawn() === "open";
  });
  const toggle = () => {
    setPressed({ session: props.session ?? null, open: !open() });
  };
  const through = (run: RunBelief) =>
    props.boundary?.kind === "forked" && props.boundary.mother === run.run
      ? props.boundary.origin.at_seq
      : null;
  return { drawn, open, toggle, through };
}

export function Divider(
  props: EarlierProps & {
    readonly shown: number;
    readonly onFork: (plan: ForkPlan) => void;
    readonly onRetry: (task: string) => void;
  },
) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const { drawn, open, toggle, through } = useFold(props, () => props.shown);
  const mother = createMemo(() =>
    props.boundary?.kind === "forked" ? belief().runs[props.boundary.mother] : undefined,
  );
  const motherRounds = storeValue(() =>
    props.boundary?.kind === "forked" && props.boundary.turn === null
      ? u.conn.asking.ask({ rounds: { run: props.boundary.mother } })
      : undefined,
  );
  const lines = storeValue(() => {
    const room = mother()?.addr;
    return room == null ? undefined : u.conn.asking.ask({ sessions: { room } });
  });
  const folded = createMemo<RuleLook>(() => ({
    parts: [
      {
        kind: "fold",
        fold: {
          label: `${fill(say(lang(), "session_previous"), { n: String(props.earlier.length) })} · ${say(lang(), open() ? "session_collapse" : "session_expand")}`,
          wire: foldWire(open(), toggle),
        },
      },
    ],
    mark: "none",
    wire: {},
  }));
  const told = createMemo((): RuleLook | null => {
    const boundary = props.boundary;
    if (boundary === null) return null;
    if (boundary.kind === "forked") {
      const ancestor = mother();
      const answer = lines();
      const original = originalOf(
        boundary.mother,
        ancestor,
        answer !== undefined && "sessions" in answer ? answer.sessions.sessions : [],
      );
      const rounds = motherRounds();
      const turn =
        boundary.turn ??
        (rounds !== undefined && "rounds" in rounds
          ? ([...rounds.rounds.turns]
              .reverse()
              .find((item) => item.opened <= boundary.origin.at_seq)?.number ?? 0)
          : 0);
      const when: RulePart[] =
        boundary.at === null ? [] : [{ kind: "text", text: clock(lang(), boundary.at) }];
      return {
        parts: [
          {
            kind: "text",
            text: fill(say(lang(), "session_forked_divider"), {
              turn: String(turn),
              at: motherName(ancestor?.task ?? "") ?? say(lang(), "fork_mother"),
            }),
          },
          ...when,
          { kind: "link", text: say(lang(), "fork_back"), href: toFragment(original) },
        ],
        mark: "branch",
        wire: {},
      };
    }
    return boundary.at === null
      ? null
      : {
          parts: [
            {
              kind: "text",
              text: fill(say(lang(), "session_new_divider"), { at: clock(lang(), boundary.at) }),
            },
          ],
          mark: "none",
          wire: {},
        };
  });
  return (
    <>
      <Show when={drawn() !== "none"}>
        <Show when={open()}>
          <div class="fade">
            <For each={props.earlier} keyed={(run) => run.run}>
              {(run) => (
                <Thread
                  run={run()}
                  through={through(run())}
                  onFork={props.onFork}
                  onRetry={props.onRetry}
                />
              )}
            </For>
          </div>
        </Show>
        <div class="my-snug">
          <Rule {...folded()} />
        </div>
      </Show>
      <Show when={told()}>
        {(note) => (
          <div class="my-snug">
            <Rule {...note()} />
          </div>
        )}
      </Show>
    </>
  );
}

const THEN = "before:mr-tight before:text-text-faint before:content-['·']";

function Result(props: { readonly run: RunBelief; readonly through: Seq | null }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const outcome = createMemo(() => outcomeOf(props.run));
  const asked = storeValue(() => u.conn.asking.ask({ rounds: { run: props.run.run } }));
  const rounds = createMemo(() => {
    const answer = asked();
    return answer !== undefined && "rounds" in answer
      ? roundsThrough(answer.rounds, props.through)
      : null;
  });
  const quote = createMemo(
    () =>
      [...(rounds()?.turns ?? [])].reverse().find((turn) => (turn.said ?? "") !== "")?.said ?? null,
  );
  const sources = createMemo(() =>
    sourcesOf(rounds()?.opening?.task ?? props.run.task ?? "", rounds()?.turns ?? []),
  );
  const base = createMemo(() => rounds()?.opened_at ?? null);
  const head = createMemo(() => lastCheckpointIn(rounds()?.turns ?? []));
  const committed = createMemo(() => head() !== null && base() !== null && head() !== base());
  const statusInk = () => {
    const final = outcome();
    return final === null ? "text-accent" : OUTCOME_INK[final];
  };
  const statusGlyph = () => {
    const final = outcome();
    return final === null ? "pulse" : OUTCOME_GLYPH[final];
  };
  const why = () => (props.run.doing.kind === "frozen" ? (props.run.doing.completion ?? "") : "");
  return (
    <li class="mb-wide flex flex-col gap-tight">
      <div class="flex items-baseline gap-base">
        <span class="w-figure shrink-0 font-mono text-note text-text-faint">
          {props.run.started === null ? "" : hhmm(props.run.started)}
        </span>
        <span class={["shrink-0 self-center", statusInk()]}>
          <Glyph name={statusGlyph()} size="sm" />
        </span>
        <span class="min-w-0 flex-1 text-body text-text">
          {withoutSources(props.run.task ?? props.run.run)}
        </span>
      </div>
      <div class="ml-figure flex flex-col gap-tight border-l border-edge pl-base">
        <Show when={quote()}>
          {(text) => (
            <div class="pl-snug text-body text-text">
              <Grounded text={text()} sources={sources()} />
            </div>
          )}
        </Show>
        <p class="flex flex-wrap items-baseline gap-tight text-note text-text-quiet">
          <Switch
            fallback={
              <>
                <span class="text-text">
                  {fill(
                    say(
                      lang(),
                      outcome() === "failed" ? "results_block_failed" : "results_block_stopped",
                    ),
                    { why: why() },
                  )}
                </span>
                <Show
                  when={committed() ? base() : null}
                  fallback={<span class={THEN}>{say(lang(), "results_block_nothing")}</span>}
                >
                  {(from) => (
                    <span class={THEN}>
                      <Produced base={from()} head={head()} />
                    </span>
                  )}
                </Show>
              </>
            }
          >
            <Match when={outcome() === "done"}>
              <span class="text-text">{say(lang(), "results_block_done")}</span>
              <Show when={base()}>
                {(from) => (
                  <span class={THEN}>
                    <Produced base={from()} head={committed() ? head() : null} />
                  </span>
                )}
              </Show>
              <Show when={props.run.pr}>
                {(pr) => (
                  <span class={THEN}>{fill(say(lang(), "results_row_pr"), { pr: pr() })}</span>
                )}
              </Show>
            </Match>
            <Match when={outcome() === "waiting"}>
              <span class="text-alert">{say(lang(), "results_waiting")}</span>
              <Show when={props.run.ask}>
                {(ask) => (
                  <span class={["text-text", THEN]}>
                    {fill(say(lang(), "results_row_ask"), { ask: ask() })}
                  </span>
                )}
              </Show>
            </Match>
            <Match when={outcome() === null && props.run.doing.kind !== "frozen"}>
              <span class="text-accent">{say(lang(), "results_block_working")}</span>
              <span class={["text-text", THEN]}>
                <AsideLink
                  text={say(lang(), "results_block_open")}
                  href={toFragment({ kind: "run", run: props.run.run })}
                />
              </span>
            </Match>
          </Switch>
        </p>
      </div>
    </li>
  );
}

export function Stream(props: EarlierProps & { readonly shown: readonly RunBelief[] }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const { drawn, open, toggle, through } = useFold(props, () => props.shown.length);
  const heading = createMemo(() => {
    const boundary = props.boundary;
    return boundary?.kind === "forked"
      ? fill(say(lang(), "results_session_forked"), {
          mother:
            motherName(belief().runs[boundary.mother]?.task ?? "") ?? say(lang(), "fork_mother"),
          turn: String(boundary.turn),
        })
      : say(lang(), "results_session_this");
  });
  const earlier = () => say(lang(), "results_session_earlier");
  return (
    <>
      <p class="mb-wide text-note text-text-faint">
        {fill(say(lang(), "results_room_counts"), {
          sessions: String(Number(props.shown.length > 0) + Number(props.earlier.length > 0)),
          runs: String(props.shown.length + props.earlier.length),
        })}
      </p>
      <Show when={props.shown.length > 0}>
        <section class="mb-section" aria-label={heading()}>
          <h2 class="mb-base text-note text-text-quiet">{heading()}</h2>
          <ul>
            <For each={[...props.shown].reverse()} keyed={(run) => run.run}>
              {(run) => <Result run={run()} through={through(run())} />}
            </For>
          </ul>
        </section>
      </Show>
      <Show when={drawn() !== "none"}>
        <section class="mb-section" aria-label={earlier()}>
          <h2 class="mb-base text-note text-text-quiet">
            <Fold
              label={`${earlier()} · ${say(lang(), open() ? "session_collapse" : "session_expand")}`}
              wire={foldWire(open(), toggle)}
            />
          </h2>
          <Show when={open()}>
            <ul>
              <For each={[...props.earlier].reverse()} keyed={(run) => run.run}>
                {(run) => <Result run={run()} through={through(run())} />}
              </For>
            </ul>
          </Show>
        </section>
      </Show>
    </>
  );
}
