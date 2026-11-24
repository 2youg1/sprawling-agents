// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { newestWorking } from "../../core/belief/live";
import { cancel } from "../../core/commands";
import { sendingInto } from "../../core/doing";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import type { Stretch } from "../../core/stretches";
import { ui } from "../../ui";
import type { Address, Seq } from "../../wire";
import Button from "../parts/button.solid";
import AddressLine from "./address_line_view";
import { noteOf } from "./branch_note";
import { branchInto } from "./branching";
import Composer from "./composer_view";
import { draftPlace } from "./draft";
import type { ForkPlan } from "./forking";
import Scroller from "./scroller";
import { drawsAddress, factsAt, type Seat } from "./seat";
import Thread from "./thread";

export interface PastProps {
  readonly address: Address;
  readonly session: Seq;
  readonly stretch: Stretch | null;
  readonly band: boolean;
  readonly seat: Seat;
}

export default function Past(props: PastProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const branching = storeValue(u.branches.held);
  const live = createMemo(() =>
    props.stretch?.runs.filter((run) => run.doing.kind !== "frozen").at(-1),
  );
  const placeholder = createMemo(() =>
    fill(say(lang(), "talk_placeholder_room"), { room: props.address }),
  );
  const place = createMemo(() =>
    props.stretch === null ? props.address : `${props.address}:${String(props.stretch.line.began)}`,
  );
  const note = createMemo(() => {
    const branch = branchInto(branching(), props.address);
    const run = branch.kind === "refused" ? newestWorking(belief(), props.address) : undefined;
    const stop = run === undefined ? null : () => u.send(cancel(run.run));
    return noteOf(branch, props.address, lang(), stop);
  });

  function fork(plan: ForkPlan): void {
    const address = props.address;
    u.branches.open(address, { kind: "branch", origin: plan.origin }, () => {
      if (plan.draft !== null) {
        u.prefs.setDraft(draftPlace(address, belief().sessions[address] ?? null), plan.draft);
      }
    });
  }

  function stop(): boolean {
    const run = live();
    return run !== undefined && u.send(cancel(run.run));
  }

  return (
    <div class={["flex min-h-0 flex-col", !props.band && "h-full"]}>
      <Show when={!props.band && drawsAddress(props.seat)}>
        <AddressLine address={props.address} session={props.session} />
      </Show>
      <Show when={!props.band}>
        <Scroller empty={false} rejoined={0} place={place()}>
          <For each={props.stretch?.runs ?? []} keyed={(run) => run.run}>
            {(run, at) => (
              <Thread run={run()} facts={factsAt(props.seat, at(), run().run)} onFork={fork} />
            )}
          </For>
        </Scroller>
      </Show>
      <div class="flex shrink-0 flex-col gap-snug border-t border-edge-panel pt-base">
        <Show when={props.stretch}>
          {(stretch) => (
            <Composer
              placeholder={placeholder()}
              draft={draftPlace(props.address, props.session)}
              room={props.address}
              contextRun={stretch().runs.at(-1)?.run}
              conversation={{
                room: props.address,
                session: props.session,
                run: live()?.run,
                historical: true,
                imageInput: live()?.imageInput,
              }}
              started={stretch().runs.length > 0}
              sending={sendingInto(live()?.doing)}
              onStop={stop}
            />
          )}
        </Show>
        <p class="text-note text-text-quiet">
          <a
            href={toFragment({ kind: "talk", address: props.address })}
            class="underline decoration-from-font underline-offset-2 transition-colors ease-leave hover:text-text hover:ease-arrive"
          >
            {say(lang(), "talk_current")}
          </a>
        </p>
        <div class="flex flex-wrap items-center gap-base">
          <p class="text-note text-text-quiet" aria-live="polite">
            {note()?.said ?? ""}
          </p>
          <Show when={note()?.stop}>
            {(action) => (
              <Button label={action().label} name={action().name} onPress={action().onPress} />
            )}
          </Show>
        </div>
      </div>
    </div>
  );
}
