// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import { deliverable, momentOf, urgencyOf, wakeAt } from "../core/deferral";
import { say } from "../core/lang";
import { recoveryFor } from "../core/recovering";
import { storeValue } from "../core/solid_store";
import { get } from "../core/store";
import { ui } from "../ui";
import type { AxError } from "../wire";
import { DiagnosticView } from "./diagnostic_view";
import { attending } from "./mailbox/attention";
import { recover, recoveryLabel, recoveryWhy, type Refused } from "./notice_recovery";
import Button from "./parts/button.solid";
import Notice from "./parts/notice_view";
import { held as heldBy, released, within, type Clock, type Holder, type Stand } from "./refusal";
import { branchClaims } from "./talk/branching";
import { claims } from "./talk/handing";
import "./refusal.css";

const VISIBLE = 3;
const LIFE = 8_000;

type Said =
  { readonly kind: "refused"; readonly refused: Refused } | { readonly kind: "no_run_in_front" };
interface Toast {
  readonly id: number;
  readonly said: Said;
  readonly gone: boolean;
}

export interface RefusalViewProps {
  readonly stopsWithoutRun: number;
}

export default function RefusalView(props: RefusalViewProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const [toasts, setToasts] = createSignal<readonly Toast[]>([]);
  const [stand, setStand] = createSignal<Stand>({ kind: "foot" });
  let follow = 0;
  let heard: AxError | null = null;
  let answered = 0;
  let held: Refused[] = [];
  let wake: number | undefined;
  const timers = new Map<number, number>();
  const clocks = new Map<number, Clock>();
  const attention = attending(u.conversing, u.now, release);
  onCleanup(() => {
    attention.stop();
    window.clearTimeout(wake);
    for (const timer of timers.values()) window.clearTimeout(timer);
  });

  function stop(id: number): void {
    const timer = timers.get(id);
    if (timer === undefined) return;
    window.clearTimeout(timer);
    timers.delete(id);
  }
  function leave(id: number): void {
    stop(id);
    clocks.delete(id);
    setToasts((held) => held.map((toast) => (toast.id === id ? { ...toast, gone: true } : toast)));
  }
  function arm(id: number): void {
    const clock = clocks.get(id);
    if (clock === undefined) return;
    stop(id);
    clocks.set(id, { ...clock, since: u.now() });
    timers.set(
      id,
      window.setTimeout(() => {
        leave(id);
      }, clock.left),
    );
  }
  function hold(id: number, by: Holder): void {
    const clock = clocks.get(id);
    if (clock === undefined) return;
    stop(id);
    clocks.set(id, heldBy(clock, by, u.now()));
  }
  function letGo(id: number, by: Holder): void {
    const clock = clocks.get(id);
    if (clock === undefined) return;
    const after = released(clock, by, u.now());
    clocks.set(id, after.clock);
    if (after.runs) arm(id);
  }
  function push(said: Said): void {
    const kept = toasts().filter((toast) => !toast.gone);
    const present = new Set(kept.map((toast) => toast.id));
    for (const oldest of kept.slice(0, Math.max(0, kept.length + 1 - VISIBLE))) leave(oldest.id);
    const id = follow;
    follow += 1;
    clocks.set(id, { left: LIFE, since: u.now(), holders: [] });
    setToasts((current) => [
      ...current.filter((toast) => present.has(toast.id)),
      { id, said, gone: false },
    ]);
    arm(id);
  }
  function release(): void {
    window.clearTimeout(wake);
    const notices = get(u.conn.belief).notices;
    held = held.filter((refused) =>
      notices.some((notice) => notice.error === refused.error && !notice.seen),
    );
    if (held.length === 0) return;
    const seen = attention.read();
    const now = u.now();
    if (momentOf(seen, now) !== null) {
      for (const refused of held) push({ kind: "refused", refused });
      held = [];
      return;
    }
    const due = wakeAt(seen, now);
    if (due !== null) wake = window.setTimeout(release, due - now);
  }

  createEffect(belief, (now) => {
    const refusal = now.refusal;
    if (refusal === null || refusal === heard) return;
    heard = refusal;
    if (claims(get(u.conversing), refusal, now) || branchClaims(get(u.branches.held), refusal))
      return;
    const refused: Refused = {
      error: refusal,
      about: now.notices.find((notice) => notice.error === refusal)?.about ?? null,
    };
    if (deliverable(urgencyOf(refusal), attention.read(), u.now())) {
      push({ kind: "refused", refused });
      return;
    }
    held = [...held, refused];
    release();
  });
  createEffect(
    () => props.stopsWithoutRun,
    (stops) => {
      if (stops === answered) return;
      answered = stops;
      push({ kind: "no_run_in_front" });
    },
  );
  const standing = createMemo(() => toasts().some((toast) => !toast.gone));
  createEffect(standing, (visible) => {
    if (!visible) return;
    const box = [...document.querySelectorAll("main textarea")].find(
      (field) => field.getClientRects().length > 0 && field.closest("[inert]") === null,
    );
    const form = box?.closest("form") ?? null;
    if (form === null) {
      setStand({ kind: "foot" });
      return;
    }
    const measure = (): void => {
      const rect = form.getBoundingClientRect();
      setStand(
        rect.width === 0
          ? { kind: "foot" }
          : {
              kind: "composer",
              left: rect.left,
              width: rect.width,
              bottom: window.innerHeight - rect.top,
            },
      );
    };
    measure();
    const watching = new ResizeObserver(measure);
    watching.observe(form);
    window.addEventListener("resize", measure);
    return () => {
      watching.disconnect();
      window.removeEventListener("resize", measure);
    };
  });

  function actions(id: number, refused: Refused) {
    return (
      <div class="flex flex-col gap-tight">
        <DiagnosticView error={refused.error} lang={lang()} />
        <Button
          tone="quiet"
          label={say(lang(), "dismiss")}
          onPress={() => {
            u.conn.dismissRefusal();
            leave(id);
          }}
        />
        <For
          each={recoveryFor(refused.error)}
          keyed={(recovery) => recoveryLabel(recovery, lang())}
        >
          {(recovery) => {
            const why = createMemo(() => {
              const reason = recoveryWhy(u, recovery(), refused);
              return reason === undefined ? {} : { why: say(lang(), reason) };
            });
            return (
              <Button
                tone="quiet"
                label={recoveryLabel(recovery(), lang())}
                {...why()}
                onPress={() => {
                  recover(u, recovery(), refused);
                }}
              />
            );
          }}
        </For>
      </div>
    );
  }
  const placed = createMemo(() => {
    const where = stand();
    return where.kind === "composer"
      ? {
          left: `${String(where.left)}px`,
          width: `${String(where.width)}px`,
          bottom: `${String(where.bottom)}px`,
        }
      : undefined;
  });

  return (
    <ul
      class={["stack flex flex-col items-stretch gap-snug", `refusal-${stand().kind}`]}
      style={placed()}
    >
      <For each={toasts()} keyed={(toast) => toast.id}>
        {(toast) => {
          const refused = createMemo(() => {
            const said = toast().said;
            return said.kind === "refused" ? said.refused : null;
          });
          return (
            <li
              class={[
                "transition-[opacity,display] transition-discrete duration-panel still:transition-none",
                toast().gone ? "hidden opacity-0 ease-leave" : "opacity-100 ease-arrive",
              ]}
              onMouseEnter={() => hold(toast().id, "pointer")}
              onMouseLeave={() => letGo(toast().id, "pointer")}
              onFocusIn={() => hold(toast().id, "focus")}
              onFocusOut={(event) => {
                if (!within(event)) letGo(toast().id, "focus");
              }}
            >
              <Show
                when={refused()}
                fallback={
                  <Notice
                    seat="toast"
                    weight="info"
                    heading="no_run_in_front"
                    next="stop_whole_city"
                    actions={() => (
                      <Button
                        tone="quiet"
                        label={say(lang(), "dismiss")}
                        onPress={() => leave(toast().id)}
                      />
                    )}
                  />
                }
              >
                {(entry) => (
                  <Notice
                    seat="toast"
                    weight="alert"
                    action={entry().error.action}
                    code={entry().error.code}
                    subject={entry().error.subject}
                    recovery={entry().error.recovery}
                    actions={() => actions(toast().id, entry())}
                  />
                )}
              </Show>
            </li>
          );
        }}
      </For>
    </ul>
  );
}
