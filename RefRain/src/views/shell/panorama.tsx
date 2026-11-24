// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, Show } from "solid-js";

import { fill, say, type Lang } from "../../core/lang";
import { MAYOR, toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { scopedUi, ui } from "../../ui";
import type { Address, RoundsAnswer, RunId } from "../../wire";
import Glyph from "../parts/glyph.solid";
import Tabs from "../parts/tabs.solid";
import { commitStateOf, sessionRun } from "../world/chosen";
import City from "../world/city.solid";
import { Commits } from "../world/commits_view";
import { Files } from "../world/files_view";
import { SessionSheet } from "../world/sheet_view";

interface RunSurfaceProps {
  readonly here: Address;
  readonly run: RunId | null;
  readonly lang: Lang;
  readonly compact: boolean;
}

function RunSurface(props: RunSurfaceProps) {
  const asking = ui().conn.asking;
  const answer = storeValue(() => {
    const run = props.run;
    return run === null ? undefined : asking.ask({ rounds: { run } });
  });
  const rounds = createMemo<RoundsAnswer | undefined>(() => {
    const held = answer();
    return held !== undefined && "rounds" in held ? held.rounds : undefined;
  });
  const worktree = () => rounds()?.worktree;
  const base = () => rounds()?.opened_at;
  return (
    <>
      <div
        class={[
          "flex shrink-0 items-baseline justify-between gap-pane",
          { "mb-snug": props.compact },
        ]}
      >
        <p class="flex min-w-0 items-baseline gap-snug text-note text-text-quiet">
          <Show when={worktree()}>
            {(name) => (
              <span class="min-w-0 truncate">
                {fill(say(props.lang, "world_worktree"), { name: name() })}
              </span>
            )}
          </Show>
          <Show when={worktree() != null && base() != null}>
            <span aria-hidden="true">·</span>
          </Show>
          <Show when={base()}>
            {(oid) => (
              <span class="shrink-0">
                {fill(say(props.lang, "world_based_on"), { oid: oid().slice(0, 7) })}
              </span>
            )}
          </Show>
        </p>
        <Show when={props.run}>
          {(run) => (
            <a
              class="shrink-0 text-note text-accent hover:text-accent-hover"
              href={toFragment({ kind: "run", run: run() })}
            >
              {say(props.lang, "world_open_run")}
            </a>
          )}
        </Show>
      </div>
      <Show
        when={props.run}
        fallback={
          <div class="mt-snug border-t border-edge">
            <div class="flex w-full flex-col items-start gap-base px-wide py-wide text-left">
              <Glyph name="ring" class="text-text-faint" />
              <p class="max-w-measure text-note text-text-quiet">
                {say(props.lang, "world_no_session")}
              </p>
            </div>
          </div>
        }
      >
        {(run) => (
          <div class={{ "mt-snug flex min-h-0 flex-1 flex-col": !props.compact }}>
            <SessionSheet here={props.here} run={run()} rounds={rounds()} />
          </div>
        )}
      </Show>
    </>
  );
}

export function GaugeStrip(props: {
  readonly here: Address;
  readonly run: RunId | null;
  readonly lang: Lang;
}) {
  return (
    <Show when={props.run}>
      {(run) => (
        <div class="shrink-0" role="group" aria-label={say(props.lang, "world_session")}>
          <RunSurface here={props.here} run={run()} lang={props.lang} compact />
        </div>
      )}
    </Show>
  );
}

export function SessionPane(props: {
  readonly here: Address;
  readonly shown?: RunId | null | undefined;
  readonly lang: Lang;
}) {
  const belief = storeValue(ui().conn.belief);
  const pick = storeValue(commitStateOf(scopedUi()));
  const run = createMemo(() =>
    props.shown === undefined ? sessionRun(belief(), props.here, pick()) : props.shown,
  );
  return (
    <section
      class="flex min-h-0 flex-1 flex-col overflow-hidden"
      aria-label={say(props.lang, "world_session")}
    >
      <RunSurface here={props.here} run={run()} lang={props.lang} compact={false} />
    </section>
  );
}

export function PlacePane(props: { readonly here: Address; readonly lang: Lang }) {
  const [reading, setReading] = createSignal("commits");
  const label = () => say(props.lang, "world_place");
  const lenses = createMemo(() => [
    { id: "commits", label: say(props.lang, "world_commits") },
    { id: "place", label: say(props.lang, props.here === MAYOR ? "world_city" : "world_files") },
  ]);
  return (
    <section
      class="refrain-place-pane flex min-h-0 flex-1 flex-col overflow-hidden"
      aria-label={label()}
    >
      <Tabs
        label={label()}
        lenses={lenses()}
        current={reading()}
        onPick={setReading}
        panel={(lens) => (
          <Show
            when={lens.id === "commits"}
            fallback={
              <Show when={props.here !== MAYOR} fallback={<City />}>
                <Files here={props.here} />
              </Show>
            }
          >
            <Commits here={props.here} />
          </Show>
        )}
      />
    </section>
  );
}
