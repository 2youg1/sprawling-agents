// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, onCleanup, Show, untrack } from "solid-js";
import { QUERIES } from "../core/asking";
import { heldIn } from "../core/belief/rooms";
import { cancel, dispatch, steer } from "../core/commands";
import { sendingInto } from "../core/doing";
import { forkAsked } from "../core/forking";
import { fill, say } from "../core/lang";
import { landingOf, sentFrom, type Landing, type Sent } from "../core/landing";
import { drawsCalls } from "../core/results";
import { MAYOR } from "../core/route";
import { storeValue } from "../core/solid_store";
import { lineIn } from "../core/stretches";
import { hhmmss } from "../core/time";
import { ui } from "../ui";
import type { Address, AxError } from "../wire";
import Button from "./parts/button.solid";
import Showing from "./shared/showing.solid";
import AddressLine from "./talk/address_line_view";
import { noteOf } from "./talk/branch_note";
import { branchInto } from "./talk/branching";
import Composer from "./talk/composer_view";
import { Delivered, Landed } from "./talk/delivery_view";
import { delivered, NONE, sent as leftTheBox, type Delivery, type Heard } from "./talk/delivery";
import { Divider, Stream } from "./talk/earlier_view";
import { draftPlace } from "./talk/draft";
import Failed from "./talk/failed_view";
import Forking from "./talk/forking_view";
import { roundsThrough, type Boundary, type ForkPlan } from "./talk/forking";
import { hand, IDLE, newestSeq, NOT_CONVERSING, settle, type Handing } from "./talk/handing";
import Inbox from "./talk/inbox_view";
import { called } from "./talk/naming";
import Scroller from "./talk/scroller";
import { drawsAddress, factsAt, type Seat } from "./talk/seat";
import Thread from "./talk/thread";
import Waiting from "./talk/waiting_view";

export interface TalkProps {
  readonly address: Address;
  readonly band: boolean;
  readonly seat: Seat;
}

export default function Talk(props: TalkProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const effort = storeValue(u.effort);
  const policy = storeValue(u.policy);
  const held = storeValue(u.prefs.held);
  const branching = storeValue(u.branches.held);
  const link = storeValue(u.conn.state);
  const asks = storeValue(forkAsked);
  const runs = createMemo(() =>
    heldIn(belief(), props.address).filter((run) => run.sessionBegan === undefined),
  );
  const live = createMemo(() =>
    belief()
      .live.filter((run) => run.addr === props.address && run.sessionBegan === undefined)
      .at(-1),
  );
  const began = createMemo(() => belief().sessions[props.address] ?? null);
  const [story, setStory] = createSignal<Boundary | null>(null);
  const [sent, setSent] = createSignal<Sent | null>(null);
  const [delivery, setDelivery] = createSignal<Delivery>(NONE);
  const [refused, setRefused] = createSignal<{
    readonly words: string;
    readonly error: AxError;
  } | null>(null);
  const [picking, setPicking] = createSignal(false);
  const [rejoined, setRejoined] = createSignal(0);
  let ours = false;
  let seen = untrack(began);
  let lastAsk = 0;
  let handing: Handing = IDLE;

  function stop(): boolean {
    const run = live();
    return run !== undefined && u.send(cancel(run.run));
  }

  const branchNote = createMemo(() =>
    noteOf(
      branchInto(branching(), props.address),
      props.address,
      lang(),
      live() === undefined ? null : stop,
    ),
  );
  const identity = storeValue(u.conn.asking.ask(QUERIES.identity));
  const stated = createMemo(() => {
    const answer = identity();
    return answer !== undefined && "identity" in answer && "stated" in answer.identity
      ? answer.identity.stated.mayor
      : null;
  });
  const who = createMemo(() => called(props.address, stated(), lang()));
  const sessionAnswer = storeValue(() => u.conn.asking.ask({ sessions: { room: props.address } }));
  const currentLine = createMemo(() => {
    const answer = sessionAnswer();
    return lineIn(
      answer !== undefined && "sessions" in answer ? answer.sessions : undefined,
      undefined,
    );
  });
  const origin = createMemo(() => {
    const line = currentLine();
    return line?.began === began() ? (line.branched_from ?? null) : null;
  });
  const mother = createMemo(() => {
    const source = origin();
    return source === null ? undefined : belief().runs[source.run];
  });
  const earlier = createMemo(() => {
    if (origin() !== null) {
      const from = mother();
      return from === undefined ? [] : [from];
    }
    const start = began();
    return start === null ? [] : runs().filter((run) => run.lastSeq <= start);
  });
  const shown = createMemo(() => {
    const start = began();
    return start === null ? runs() : runs().filter((run) => run.lastSeq > start);
  });
  const changed = createMemo(() => {
    const policyChange = belief().policies[props.address] ?? null;
    const start = began();
    return policyChange === null || (start !== null && policyChange.seq <= start)
      ? null
      : policyChange;
  });
  const policyAt = createMemo(() => {
    const change = changed();
    return change === null ? -1 : shown().findIndex((run) => (run.started ?? 0) > change.at);
  });
  const policyLine = createMemo(() => {
    const change = changed();
    return change === null
      ? null
      : fill(say(lang(), "talk_policy_changed"), {
          mode: say(lang(), `mode_${change.policy.mode}`),
          write: say(lang(), `admission_value_${change.policy.write}`),
        });
  });
  const current = createMemo(() => live() ?? shown().at(-1) ?? mother());
  const rawAnswer = storeValue(() => {
    const run = current()?.run;
    return run === undefined ? null : u.conn.asking.ask({ rounds: { run } });
  });
  const answer = createMemo(() => {
    const response = rawAnswer();
    return response !== undefined && "rounds" in response
      ? roundsThrough(
          response.rounds,
          live() === undefined && shown().length === 0 ? (origin()?.at_seq ?? null) : null,
        )
      : undefined;
  });
  const said = createMemo(() =>
    answer()
      ?.turns.filter((turn) => typeof turn.said === "string")
      .at(-1),
  );

  const boundary = createMemo<Boundary | null>(() => {
    const source = origin();
    const held = story();
    if (source === null) return held;
    const local = held?.kind === "forked" && held.mother === source.run ? held : null;
    return {
      kind: "forked",
      at: local?.at ?? null,
      turn: local?.turn ?? null,
      mother: source.run,
      origin: source,
    };
  });
  createEffect(began, (opened) => {
    if (opened === null || opened === seen) return;
    seen = opened;
    if (ours) ours = false;
    else setStory({ kind: "opened", at: u.now() });
  });

  const landing = createMemo<Landing>(() => {
    const last = sent();
    if (last?.from !== props.address) return { kind: "pending" };
    const known = belief().runs;
    const found = landingOf(last, known);
    return found.kind === "elsewhere" && known[found.run]?.doing.kind === "frozen"
      ? { kind: "pending" }
      : found;
  });
  function hear(): Heard {
    const now = belief();
    return { live: link().kind === "live", newest: newestSeq(now), refusal: now.refusal };
  }
  createEffect(
    () => ({ prior: delivery(), heard: hear(), landing: landing(), sent: sent() }),
    ({ prior, heard, landing, sent }) => {
      if (prior.kind === "accepted") {
        if (sent === null || landing.kind !== "pending") setDelivery(NONE);
      } else if (prior.kind !== "none") setDelivery(delivered(prior, heard));
    },
  );
  const echo = createMemo(() => {
    const showing = delivery();
    return showing.kind === "none" ? null : showing;
  });
  const blank = createMemo(() => shown().length === 0 && origin() === null && echo() === null);
  const addressed = createMemo(() => !props.band && drawsAddress(props.seat));
  const pendingFork = createMemo(() => {
    const here = story();
    return shown().length === 0 && here?.kind === "forked" ? here : null;
  });
  const placeholder = createMemo(() =>
    props.address === MAYOR && stated() === null
      ? say(lang(), "talk_placeholder_mayor")
      : fill(say(lang(), "talk_placeholder_room"), { room: who() }),
  );

  function awaitAnswer(words: string): void {
    setRefused(null);
    handing = hand(words, belief());
    u.conversing.set({ kind: "waiting", handing });
  }
  function noteDispatch(task: string): void {
    setSent(sentFrom(props.address, task, belief().runs));
  }
  function send(words: string): boolean {
    const run = live();
    if (run !== undefined) {
      const sent = u.send(steer(run.run, words));
      if (sent) {
        setSent(null);
        setDelivery(leftTheBox(words, hear()));
        awaitAnswer(words);
      }
      return sent;
    }
    const sent = u.send(
      dispatch({
        addr: props.address,
        task: words,
        goal: say(lang(), "talk_goal"),
        effort: effort(),
        policy: policy(),
      }),
    );
    if (sent) {
      noteDispatch(words);
      setDelivery(leftTheBox(words, hear()));
      awaitAnswer(words);
    }
    return sent;
  }
  createEffect(belief, (now) => {
    const prior = handing;
    const next = settle(prior, now, "");
    handing = next.handing;
    if (prior.kind !== "handed" || next.handing.kind !== "idle") return;
    const error = next.box === null ? null : now.refusal;
    if (error !== null) setRefused({ words: prior.words, error });
    u.conversing.set(error === null ? NOT_CONVERSING : { kind: "answered", refusal: error });
  });
  onCleanup(() => {
    u.conversing.set(NOT_CONVERSING);
  });

  function doFork(plan: ForkPlan): void {
    setPicking(false);
    const address = props.address;
    u.branches.open(address, { kind: "branch", origin: plan.origin }, () => {
      if (plan.draft !== null)
        u.prefs.setDraft(draftPlace(address, belief().sessions[address] ?? null), plan.draft);
      ours = true;
      setStory({
        kind: "forked",
        at: u.now(),
        turn: plan.turn,
        mother: plan.mother,
        origin: plan.origin,
      });
      setRejoined((count) => count + 1);
    });
  }
  createEffect(asks, (requested) => {
    if (requested === lastAsk) return;
    lastAsk = requested;
    setPicking(true);
  });

  function addressInBox() {
    return <AddressLine address={props.address} at="box" />;
  }
  function lastSaid() {
    return (
      <Show when={said()}>
        {(line) => (
          <div class="mb-snug flex flex-col gap-tight">
            <div class="flex items-baseline gap-base text-note text-text-faint">
              <span class="font-label text-text">
                {called(props.address, answer()?.opening?.names?.mayor, lang())}
              </span>
              <span class="figure">{hhmmss(line().t)}</span>
            </div>
            <p class="truncate text-body text-text">{line().said}</p>
          </div>
        )}
      </Show>
    );
  }

  return (
    <div
      class={["flex min-h-0 flex-col", !props.band && "h-full"]}
      style={{ "container-type": props.band ? undefined : "size" }}
    >
      <Show when={!props.band}>
        <Scroller empty={blank()} rejoined={rejoined()} place={props.address}>
          <Show when={shown().length > 0 || origin() !== null}>
            <div class="mb-base flex justify-end">
              <Showing />
            </div>
            <Show
              when={drawsCalls(held().showing)}
              fallback={
                <Stream
                  session={began()}
                  shown={shown()}
                  earlier={earlier()}
                  boundary={boundary()}
                />
              }
            >
              <Divider
                session={began()}
                earlier={earlier()}
                shown={shown().length}
                boundary={boundary()}
                onFork={doFork}
                onRetry={send}
              />
              <For each={shown()} keyed={(run) => run.run}>
                {(run, at) => (
                  <>
                    <Show when={policyAt() === at() ? policyLine() : null}>
                      {(line) => (
                        <p class="my-snug text-center text-note text-text-faint">{line()}</p>
                      )}
                    </Show>
                    <Thread
                      run={run()}
                      facts={factsAt(props.seat, at(), run().run)}
                      onFork={doFork}
                      onRetry={send}
                    />
                  </>
                )}
              </For>
              <Show when={policyAt() === -1 ? policyLine() : null}>
                {(line) => <p class="my-snug text-center text-note text-text-faint">{line()}</p>}
              </Show>
            </Show>
          </Show>
          <Show when={echo()}>{(value) => <Delivered delivery={value()} />}</Show>
          <Waiting />
          <Inbox addr={props.address} />
        </Scroller>
      </Show>
      <div
        class="relative shrink-0 transition-transform duration-page"
        style={{
          transform:
            !props.band && blank()
              ? "translateY(calc(-50cqh + 50% + var(--spacing-margin)))"
              : undefined,
        }}
      >
        <Show when={branchNote()}>
          {(note) => (
            <div class="mb-tight flex flex-wrap items-center gap-base">
              <p class="text-note text-text-faint" role="status">
                {note().said}
              </p>
              <Show when={note().stop}>
                {(action) => (
                  <Button label={action().label} name={action().name} onPress={action().onPress} />
                )}
              </Show>
            </div>
          )}
        </Show>
        <Show when={pendingFork()}>
          {(boundary) => (
            <p class="mb-tight text-note text-text-faint" role="status">
              {fill(say(lang(), "fork_pending"), { turn: String(boundary().turn) })}
            </p>
          )}
        </Show>
        <For each={[story()]} keyed={(story) => story}>
          {() => (
            <>
              <Landed landing={landing()} />
              <Show when={refused()}>
                {(fault) => (
                  <Failed
                    what={say(
                      lang(),
                      fault().error.code === "E_MODEL_UNCHOSEN"
                        ? "talk_failed_model"
                        : "talk_failed_refused",
                    )}
                    error={fault().error}
                    onRetry={() => {
                      const words = refused()?.words;
                      setRefused(null);
                      if (words !== undefined) return send(words);
                      return false;
                    }}
                  />
                )}
              </Show>
              <Composer
                placeholder={placeholder()}
                started={echo() !== null}
                sending={sendingInto(live()?.doing)}
                draft={draftPlace(props.address, began())}
                conversation={{
                  room: props.address,
                  session: began(),
                  run: live()?.run,
                  imageInput: live()?.imageInput,
                }}
                onDispatch={noteDispatch}
                hearing={u.hearing()}
                room={props.address}
                band={props.band ? lastSaid : undefined}
                door={addressed() ? addressInBox : undefined}
                onSend={send}
                onStop={stop}
              />
            </>
          )}
        </For>
        <Show when={picking() ? answer() : undefined}>
          {(rounds) => (
            <Forking
              rounds={rounds()}
              onFork={doFork}
              onClose={() => {
                setPicking(false);
              }}
            />
          )}
        </Show>
      </div>
    </div>
  );
}
