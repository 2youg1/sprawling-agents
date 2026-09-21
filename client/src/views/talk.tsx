// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The first page: one conversation with one room, the Mayor's unless
// the address says otherwise. Every run in the room is a stretch of
// the same thread; what waits for the person is a card in the same
// thread; and the box at the bottom either steers the run that is
// going or opens the next one.
//
// An empty Mayor's office carries the three sentences the welcome
// walk used to end on: how work is handed out, where it is watched,
// and where a question waiting on an answer appears. They are here
// rather than on a fifth welcome step because each of them names
// something that is on this screen while it is being read, and
// because the first run in this room replaces them with itself.
//
// Beside the conversation, once the room is wide enough to hold two
// columns, stands what the run produced. The width that decides it is
// the main region's, asked with a container query: an open rail takes
// 232px of the window, so a layout that asked the window would put two
// columns into a space that holds one.

import { For, Show, createEffect, createMemo, createSignal, onCleanup } from "solid-js";

import { cancel, dispatch, steer } from "../core/commands";
import { sendingInto, type RunBelief } from "../core/belief";
import { MAYOR, roomOf } from "../core/route";
import type { Address, RoundsAnswer } from "../wire";
import { useCommand, useHearing, useSay, useUi } from "../ui";
import { Artifact, PANEL_ID } from "./talk/artifact";
import { Composer } from "./talk/composer";
import { Thread } from "./talk/thread";
import { NOTHING, artifactsIn, type Artifacts } from "./talk/trace";
import { Waiting } from "./talk/waiting";

export interface TalkProps {
  readonly address: Address;
}

export function Talk(props: TalkProps) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const hearing = useHearing();

  // The runs of this room, oldest first. A run whose room is not yet
  // known is not shown here rather than shown in the wrong room.
  const runs = createMemo<RunBelief[]>(() =>
    Object.values(ui.conn.belief.runs)
      .filter((run) => run.addr === props.address)
      .sort((a, b) => (a.started ?? 0) - (b.started ?? 0) || a.lastSeq - b.lastSeq),
  );
  const live = createMemo(() => [...runs()].reverse().find((run) => run.doing.kind !== "frozen"));
  const isMayor = () => props.address === MAYOR;

  // The run the panel speaks for: the one still going, or the last one
  // this room finished. The same question `Thread` asks, merged with it
  // by `asking` because the two ask it in the same words.
  const current = createMemo<RunBelief | undefined>(() => live() ?? runs().at(-1));
  const rounds = createMemo(() => {
    const run = current();
    return run === undefined ? undefined : ui.conn.asking.ask({ rounds: { run: run.run } });
  });
  const artifacts = createMemo<Artifacts>(() => {
    const held = rounds()?.();
    if (held === undefined || !("rounds" in held)) return NOTHING;
    const answer: RoundsAnswer = held.rounds;
    return artifactsIn(answer.turns);
  });
  // Whether there is a card to draw at all. Any one of the three
  // panes is enough; a run that read nothing, changed nothing and ran
  // nothing gets no card and no control to open one.
  const produced = () => {
    const held = artifacts();
    return held.read !== null || held.wrote !== null || held.terminal !== null;
  };
  // Open until the person closes it, and forgotten on reload: this
  // belongs in the person's own `[ui]` section and there is no door to
  // it yet (client-SPEC 4-27).
  const panel = () => ui.prefs.held().panel;
  const setPanel = ui.prefs.setPanel;

  const [scroller, setScroller] = createSignal<HTMLDivElement>();
  const [pinned, setPinned] = createSignal(true);
  const follow = () => {
    const box = scroller();
    if (pinned() && box !== undefined) {
      box.scrollTop = box.scrollHeight;
    }
  };
  // Keep the newest words in view while the person is at the bottom,
  // and leave them alone once they have scrolled up to read. The column
  // grows when answers land as well as when records do, so its size is
  // what is watched.
  const [column, setColumn] = createSignal<HTMLDivElement>();
  createEffect(() => {
    const held = column();
    if (held === undefined) return;
    const watcher = new ResizeObserver(() => {
      follow();
    });
    watcher.observe(held);
    onCleanup(() => {
      watcher.disconnect();
    });
  });

  const send = (text: string) => {
    const going = live();
    if (going !== undefined) {
      return command(steer(going.run, text));
    }
    return command(
      dispatch({
        addr: props.address,
        task: text,
        goal: say("talk_goal"),
        effort: ui.effort(),
      }),
    );
  };

  // One composer, drawn in the middle of an empty room and in the bar
  // once the room has a thread. Two call sites, one element: a second
  // <Composer> would carry a second draft and a second selection.
  const composer = () => (
    <Composer
      placeholder={isMayor() ? say("talk_placeholder_mayor") : say("talk_placeholder_room", { room: roomOf(props.address) })}
      sending={sendingInto(live()?.doing)}
      draft={props.address}
      hearing={hearing()}
      onSend={send}
      onStop={() => {
        const going = live();
        return going === undefined ? false : command(cancel(going.run));
      }}
    />
  );

  return (
    <div class="flex min-h-0 flex-1 flex-col @lg/page:flex-row">
      <div class="flex min-h-0 flex-1 flex-col">
        {/* The one control the panel has. It is here rather than on the
            panel because the panel is what it opens: a second control
            inside would be a second place to look for the same state. */}
        <Show when={produced()}>
          <div class="flex justify-end px-pane pt-snug">
            <button
              type="button"
              class="rounded-control px-snug py-tight text-note text-text-faint hover:bg-chrome hover:text-text-quiet"
              aria-expanded={panel()}
              aria-controls={PANEL_ID}
              onClick={() => {
                setPanel(!panel());
              }}
            >
              {panel() ? say("talk_panel_hide") : say("talk_panel_show")}
            </button>
          </div>
        </Show>
        <div
          ref={setScroller}
          class="min-h-0 flex-1 overflow-y-auto"
          onScroll={(event) => {
            const box = event.currentTarget;
            setPinned(box.scrollHeight - box.scrollTop - box.clientHeight < 48);
          }}
        >
          <div ref={setColumn} class="mx-auto flex min-h-full w-full max-w-talk flex-col justify-end px-pane pb-wide pt-wide">
            {/* The page's own name, and it is always here.

                This page had no `<h1>` at all: the empty room drew its
                title as a `<p>`, and once a thread started there was
                nothing naming the page. A reader arriving by keyboard
                or by screen reader had nothing to land on, and
                `theme.css` hangs the view transition off `main h1`, so
                route changes carried nothing across either. `xtask
                render` never caught it because it opens `#/gallery`
                and nothing else; it is the first screen of the product
                and the one page the gate does not look at.

                It is drawn as the heading of an empty room and read
                out but not drawn once the conversation has started -
                the thread is then what the page is about, and a
                standing title above it would be furniture. */}
            <h1 class={runs().length === 0 ? "sr-only" : "mb-wide text-note text-text-faint"}>
              {isMayor() ? say("talk_empty_mayor") : roomOf(props.address)}
            </h1>
            <Show when={!isMayor() && runs().length === 0}>
              <p class="mb-wide text-note text-text-faint">{props.address}</p>
            </Show>
            {/* An empty room opens with the box in the middle of the page
                and the room's own name above it, because the first thing
                asked of a person here is to say something. The box rides
                down to the bar on the first send (Composer measures the
                distance itself), so nothing about the drop lives here. */}
            <Show when={runs().length === 0}>
              <div class="my-auto flex flex-col items-center gap-base py-section text-center">
                <p class="text-heading font-heading text-text-disabled">
                  {isMayor() ? say("talk_empty_mayor") : say("talk_empty_room", { room: roomOf(props.address) })}
                </p>
                <p class="text-note text-text-faint">
                  {isMayor() ? say("talk_opening_mayor") : say("talk_opening_room", { room: roomOf(props.address) })}
                </p>
                <div class="w-full">{composer()}</div>
                {/* The three opening lines take the composer's own
                    left edge rather than their own centred one. As a
                    block of its own width inside a centred column they
                    began forty pixels to its right, which reads as two
                    columns that failed to line up rather than as one
                    thing with a note under it. */}
                <Show when={isMayor()}>
                  <ul class="w-full list-none text-left text-note text-text-faint">
                    <li>{say("talk_hint_dispatch")}</li>
                    <li class="mt-tight">{say("talk_hint_progress")}</li>
                    <li class="mt-tight">{say("talk_hint_waiting")}</li>
                  </ul>
                </Show>
              </div>
            </Show>
            <For each={runs()}>{(run) => <Thread run={run} who={roomOf(props.address)} />}</For>
            <Waiting />
          </div>
        </div>
        <Show when={runs().length > 0}>
          <div class="mx-auto w-full max-w-talk px-pane pb-pane">{composer()}</div>
        </Show>
      </div>
      <Show when={produced()}>
        <Artifact artifacts={artifacts()} open={panel()} />
      </Show>
    </div>
  );
}
