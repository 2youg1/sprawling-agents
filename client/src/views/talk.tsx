// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The first page: one conversation with one room, the Mayor's unless
// the address says otherwise. Every run in the room is a stretch of
// the same thread; what waits for the person is a card in the same
// thread; and the box at the bottom either steers the run that is
// going or opens the next one.

import { For, Show, createEffect, createMemo, createSignal, onCleanup } from "solid-js";

import { cancel, dispatch, release, steer } from "../core/commands";
import { sendingInto, type RunBelief } from "../core/belief";
import { MAYOR, roomOf } from "../core/route";
import type { Address } from "../wire";
import { useCommand, useSay, useUi } from "../ui";
import { Composer } from "./talk/composer";
import { Thread } from "./talk/thread";
import { Waiting } from "./talk/waiting";

export interface TalkProps {
  readonly address: Address;
}

export function Talk(props: TalkProps) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();

  // The runs of this room, oldest first. A run whose room is not yet
  // known is not shown here rather than shown in the wrong room.
  const runs = createMemo<RunBelief[]>(() =>
    Object.values(ui.conn.belief.runs)
      .filter((run) => run.addr === props.address)
      .sort((a, b) => (a.started ?? 0) - (b.started ?? 0) || a.lastSeq - b.lastSeq),
  );
  const live = createMemo(() => [...runs()].reverse().find((run) => run.doing.kind !== "frozen"));
  const halted = createMemo(() => ui.conn.belief.halted.includes("city"));
  const isMayor = () => props.address === MAYOR;

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
        effort: ui.prefs.effort(),
      }),
    );
  };

  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <div
        ref={setScroller}
        class="min-h-0 flex-1 overflow-y-auto"
        onScroll={(event) => {
          const box = event.currentTarget;
          setPinned(box.scrollHeight - box.scrollTop - box.clientHeight < 48);
        }}
      >
        <div ref={setColumn} class="mx-auto flex min-h-full w-full max-w-talk flex-col justify-end px-pane pb-wide pt-wide">
          <Show when={!isMayor()}>
            <p class="mb-wide text-note text-text-faint">{props.address}</p>
          </Show>
          <Show when={runs().length === 0}>
            <div class="my-auto py-section text-center">
              <p class="text-heading font-heading text-text-disabled">
                {isMayor() ? say("talk_empty_mayor") : say("talk_empty_room", { room: roomOf(props.address) })}
              </p>
            </div>
          </Show>
          <For each={runs()}>{(run) => <Thread run={run} who={roomOf(props.address)} />}</For>
          <Waiting />
        </div>
      </div>
      <div class="mx-auto w-full max-w-talk px-pane pb-pane">
        <Show when={halted()}>
          <div class="mb-snug flex items-center justify-between rounded-card bg-g1 px-base py-snug text-note text-text-quiet">
            <span>{say("talk_halted")}</span>
            <button
              type="button"
              class="rounded-control px-snug py-tight text-label text-alert hover:bg-g2"
              onClick={() => command(release("city"))}
            >
              {say("talk_release")}
            </button>
          </div>
        </Show>
        <Composer
          placeholder={isMayor() ? say("talk_placeholder_mayor") : say("talk_placeholder_room", { room: roomOf(props.address) })}
          sending={sendingInto(live()?.doing)}
          onSend={send}
          onStop={() => {
            const going = live();
            return going === undefined ? false : command(cancel(going.run));
          }}
        />
      </div>
    </div>
  );
}
