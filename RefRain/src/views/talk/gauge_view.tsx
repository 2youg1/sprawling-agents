// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { children, createMemo, createUniqueId, For, Show } from "solid-js";
import { QUERIES } from "../../core/asking";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, RunId } from "../../wire";
import { openContext } from "../inspect/open";
import ControlTip from "../parts/control_tip";
import {
  contextOf,
  lockOfConfig,
  remindersOf,
  ringOf,
  sessionRunOf,
  type Checkpoint,
  type RingLook,
} from "./gauge";
import "./marks.css";

interface Props {
  readonly room: Address | null;
  readonly run?: RunId | undefined;
  readonly historical?: boolean;
  readonly children: JSX.Element;
}

export function GaugeCheckpoint(props: Checkpoint & { readonly along: "ring" | "line" }) {
  return (
    <i
      class={["talk-gauge-checkpoint", `talk-gauge-${props.reminder}`, `talk-gauge-${props.along}`]}
      style={{ "--at": props.at }}
      aria-hidden="true"
    />
  );
}

export default function Gauge(props: Props) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const run = createMemo(() =>
    props.historical === true
      ? props.run
      : props.room === null
        ? undefined
        : sessionRunOf(belief(), props.room),
  );
  const rounds = storeValue(() => {
    const id = run();
    return id === undefined ? null : u.conn.asking.ask({ rounds: { run: id } });
  });
  const config = storeValue(() =>
    props.room === null ? null : u.conn.asking.ask({ config: { addr: props.room } }),
  );
  const ring = createMemo(() => {
    const answer = rounds();
    const models = endpoints();
    return ringOf(
      lang(),
      contextOf(
        answer !== undefined && "rounds" in answer ? answer.rounds.turns : [],
        models !== undefined && "endpoints" in models ? models.endpoints.endpoints : [],
        remindersOf(config()),
        lockOfConfig(config()),
      ),
    );
  });
  function open(): void {
    const at = run();
    if (at !== undefined) openContext({ run: at, room: props.room });
  }
  return (
    <GaugeRing ring={ring()} onOpen={run() === undefined ? null : open}>
      {props.children}
    </GaugeRing>
  );
}

export function GaugeRing(props: {
  readonly ring: RingLook | null;
  readonly onOpen: (() => void) | null;
  readonly children: JSX.Element;
}) {
  const content = children(() => props.children);
  const id = createUniqueId();
  return (
    <Show
      when={props.ring}
      fallback={<span class="grid size-key shrink-0 place-items-center">{content()}</span>}
    >
      {(look) => (
        <ControlTip id={id} text={look().reading}>
          <span class="relative grid size-key shrink-0 place-items-center">
            <Show when={props.onOpen !== null}>
              <button
                type="button"
                data-control=""
                aria-describedby={id}
                aria-label={look().meter["aria-label"]}
                aria-valuemin={look().meter["aria-valuemin"]}
                aria-valuemax={look().meter["aria-valuemax"]}
                aria-valuenow={look().meter["aria-valuenow"]}
                aria-valuetext={look().meter["aria-valuetext"]}
                class="absolute inset-0 rounded-pill transition-[background-color,scale] ease-leave hover:bg-raised-hover/40 hover:ease-arrive hover:scale-105 active:scale-(--scale-press) active:ease-arrive still:transition-none still:hover:scale-100 still:active:scale-100"
                onClick={(event) => {
                  if (event.target === event.currentTarget) props.onOpen?.();
                }}
              />
            </Show>
            <svg
              class="pointer-events-none absolute inset-0 -rotate-90"
              viewBox="0 0 40 40"
              aria-hidden="true"
            >
              <circle
                class="talk-gauge-arc fill-none stroke-text"
                cx="20"
                cy="20"
                r="18"
                stroke-width="1"
                pathLength="100"
                stroke-dasharray={look().arc.dasharray}
                stroke-dashoffset={look().arc.dashoffset}
              />
            </svg>
            <For each={look().checkpoints} keyed={(checkpoint) => checkpoint.reminder}>
              {(checkpoint) => <GaugeCheckpoint {...checkpoint()} along="ring" />}
            </For>
            <span class="relative z-1 grid place-items-center">{content()}</span>
          </span>
        </ControlTip>
      )}
    </Show>
  );
}
