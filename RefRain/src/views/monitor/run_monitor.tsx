// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createEffect, createMemo, createSignal, onSettled, untrack } from "solid-js";
import type { Tail } from "../../core/live_output";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Turn } from "../../wire";
import { lookOf, resumes, type Following, type Pane } from "./monitor";
import { traceOf, type Target } from "./trace";
import { CodeColumn, Terminal } from "./trace_view";

const TOGGLE =
  "inline-flex h-control-sm items-center rounded-control px-snug text-label transition-colors ease-leave hover:ease-arrive";
const PANE =
  "relative min-h-0 flex-1 overflow-y-auto focus-visible:outline-2 focus-visible:outline-accent";

interface Props {
  readonly turns: readonly Turn[];
  readonly tail: Tail;
  readonly live: boolean;
  readonly onDraft: (text: string) => void;
  readonly onSteer: (text: string) => void;
  readonly following?: Following | undefined;
  readonly terminal?: Pane | undefined;
}

export default function RunMonitor(props: Props) {
  const lang = storeValue(ui().lang);
  const [following, setFollowing] = createSignal<Following>(
    untrack(() => props.following ?? "following"),
  );
  const [terminal, setTerminal] = createSignal<Pane>(untrack(() => props.terminal ?? "open"));
  const trace = createMemo(() => traceOf(props.turns));
  const refs: { code?: HTMLElement; record?: HTMLElement } = {};
  function land(target: Target): void {
    if (target.kind === "file") {
      const host = refs.code;
      const file = host?.querySelector<HTMLElement>(`[data-path="${CSS.escape(target.path)}"]`);
      if (host !== undefined && file != null) host.scrollTop = file.offsetTop;
    } else {
      const host = refs.record;
      const entry = host?.querySelector<HTMLElement>(`[data-at="${String(target.at)}"]`);
      if (host !== undefined && entry != null)
        host.scrollTop = entry.offsetTop + entry.offsetHeight - host.clientHeight;
    }
  }
  createEffect(
    () => [following(), trace().latest, terminal()] as const,
    ([state, target]) => {
      if (state !== "following" || target === null) return;
      const frame = requestAnimationFrame(() => {
        land(target);
      });
      return () => {
        cancelAnimationFrame(frame);
      };
    },
  );
  onSettled(() => {
    const listener = (event: KeyboardEvent): void => {
      const typing =
        event.target instanceof HTMLElement &&
        (event.target.isContentEditable ||
          event.target instanceof HTMLInputElement ||
          event.target instanceof HTMLTextAreaElement);
      if (
        resumes({
          key: event.key,
          ctrlKey: event.ctrlKey,
          metaKey: event.metaKey,
          altKey: event.altKey,
          typing,
        })
      )
        setFollowing("following");
    };
    window.addEventListener("keydown", listener);
    return () => {
      window.removeEventListener("keydown", listener);
    };
  });
  const open = () => terminal() === "open";
  const look = createMemo(() =>
    lookOf(
      {
        following: following(),
        terminal: terminal(),
        paths: trace().files.map((file) => file.path),
      },
      lang(),
      {
        follow: setFollowing,
        fold: setTerminal,
        jump: (path) => {
          setFollowing("paused");
          land({ kind: "file", path });
        },
      },
    ),
  );
  return (
    <section class="flex min-h-0 flex-1 flex-col" aria-label={look().label}>
      <header class="flex flex-wrap items-center gap-snug border-b border-edge bg-chrome px-pane py-snug text-note">
        <span class="text-label text-text">{look().heading}</span>
        <span class="text-text-faint figure">{look().tally}</span>
        <button
          type="button"
          data-control="surface"
          aria-pressed={look().follow.held ? "true" : "false"}
          class={[
            TOGGLE,
            "ms-auto",
            look().follow.held
              ? "text-accent hover:bg-raised"
              : "bg-raised text-text hover:bg-raised-hover",
          ]}
          onClick={() => look().follow.wire.onclick()}
        >
          {look().follow.label}
        </button>
        <button
          type="button"
          data-control="surface"
          aria-expanded={open() ? "true" : "false"}
          class={[TOGGLE, "text-text-quiet hover:bg-raised hover:text-text"]}
          onClick={() => look().fold.wire.onclick()}
        >
          {look().fold.label}
        </button>
      </header>
      <div class="flex min-h-0 flex-1">
        <Show when={look().index}>
          {(index) => (
            <nav
              class="w-index shrink-0 overflow-y-auto border-e border-edge py-tight"
              aria-label={index().label}
            >
              <For each={index().files} keyed={(file) => file.path}>
                {(file) => (
                  <button
                    type="button"
                    data-control="surface"
                    class="block w-full truncate px-snug py-tight text-start font-mono text-note text-text-quiet transition-colors ease-leave hover:bg-raised hover:text-text hover:ease-arrive"
                    onClick={() => file().wire.onclick()}
                  >
                    {file().path}
                  </button>
                )}
              </For>
            </nav>
          )}
        </Show>
        <div
          role="region"
          aria-label={look().code["aria-label"]}
          tabindex={0}
          class={[PANE, open() && "run-trace-side border-e border-edge"]}
          ref={(node) => {
            refs.code = node;
          }}
          onWheel={() => look().code.onwheel()}
          onTouchMove={() => look().code.ontouchmove()}
          onKeyDown={(event) => look().code.onkeydown(event)}
        >
          <CodeColumn
            files={trace().files}
            live={props.live}
            onDraft={props.onDraft}
            onSteer={props.onSteer}
          />
        </div>
        <Show when={look().record}>
          {(record) => (
            <div
              role="region"
              aria-label={record()["aria-label"]}
              tabindex={0}
              class={[PANE, "bg-page"]}
              ref={(node) => {
                refs.record = node;
              }}
              onWheel={() => record().onwheel()}
              onTouchMove={() => record().ontouchmove()}
              onKeyDown={(event) => record().onkeydown(event)}
            >
              <Terminal entries={trace().entries} tail={props.tail} />
            </div>
          )}
        </Show>
      </div>
    </section>
  );
}
