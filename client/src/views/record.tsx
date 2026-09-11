// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one history in four lenses: the ledger as it was written, the
// archive as the buildings filed it, the recycle bin where every row
// states its own way back, and the process log beside them. Nothing
// here is folded: the record is shown as the record.
//
// The log is the one lens with nothing behind it. No frame on the wire
// carries a log line, so the lens is drawn, is inert, and says which
// piece is missing rather than showing an emptiness a reader would read
// as a quiet city.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";

import { LENSES, toFragment } from "../core/route";
import type { Lens } from "../core/route";
import { clock, hhmmss } from "../core/time";
import type { Seq } from "../wire";
import { useGo, useLang, useSay, useUi } from "../ui";
import { Button } from "./parts/button";
import { EmptyState } from "./parts/empty";
import { Notice } from "./parts/notice";
import { Path } from "./parts/path";
import { Tabs } from "./parts/tabs";

// One line of what a record carries: its scalar fields, the way a
// person skims a log.
function gist(data: Record<string, unknown>): string {
  return Object.entries(data)
    .filter(([, value]) => typeof value === "string" || typeof value === "number" || typeof value === "boolean")
    .map(([key, value]) => `${key}=${String(value)}`)
    .join("  ");
}

function Ledger() {
  const ui = useUi();
  const say = useSay();
  const lang = useLang();
  const [before, setBefore] = createSignal<Seq | null>(null);
  const [open, setOpen] = createSignal<number | null>(null);
  const history = createMemo(() => ui.conn.asking.ask({ history: { before: before(), limit: 100 } }));
  const answer = createMemo(() => {
    const held = history()();
    return held !== undefined && "history" in held ? held.history : undefined;
  });
  return (
    <Show when={answer()} fallback={<p class="text-text-disabled">…</p>}>
      {(held) => (
        <div>
          <Show when={held().earlier}>
            {(earlier) => (
              <button type="button" class="mb-base rounded-control bg-g1 px-base py-tight text-label hover:bg-g2" onClick={() => setBefore(earlier())}>
                {say("rec_earlier")}
              </button>
            )}
          </Show>
          <ul class="font-mono text-note">
            <For each={[...held().records].reverse()}>
              {(record) => (
                <li class="border-b border-g1">
                  <button
                    type="button"
                    class="flex h-step w-full items-center gap-base text-left leading-none hover:bg-g1"
                    onClick={() => setOpen((at) => (at === record.seq ? null : record.seq))}
                    title={clock(lang(), record.t)}
                  >
                    <span class="w-figure shrink-0 text-right text-text-disabled">{record.seq}</span>
                    <span class="w-figure shrink-0 whitespace-nowrap text-text-faint">{hhmmss(record.t)}</span>
                    <span class="shrink-0 text-text">{record.kind}</span>
                    <span class="shrink-0 text-text-faint">{record.addr ?? record.who}</span>
                    <span class="min-w-0 flex-1 truncate text-text-disabled">{gist(record.data)}</span>
                  </button>
                  <Show when={open() === record.seq}>
                    <pre class="mb-snug max-h-output overflow-auto rounded-card bg-g1 p-base text-text-quiet">
                      {JSON.stringify(record.data, null, 2)}
                    </pre>
                  </Show>
                </li>
              )}
            </For>
          </ul>
        </div>
      )}
    </Show>
  );
}

function Archive() {
  const ui = useUi();
  const say = useSay();
  const [needle, setNeedle] = createSignal("");
  const search = createMemo(() => ui.conn.asking.ask({ archive_search: { needle: needle() } }));
  const hits = createMemo(() => {
    const held = search()();
    return held !== undefined && "archive" in held ? held.archive.hits : undefined;
  });
  return (
    <div>
      <input
        class="mb-base w-full rounded-control bg-g1 px-base py-snug text-body outline-none placeholder:text-text-disabled"
        placeholder={say("rec_search")}
        value={needle()}
        onInput={(event) => setNeedle(event.currentTarget.value)}
      />
      <Show when={hits()} fallback={<p class="text-text-disabled">…</p>}>
        {(held) => (
          <Show when={held().length > 0} fallback={<p class="text-text-faint">{say("rec_nothing")}</p>}>
            <ul class="text-note">
              <For each={held()}>
                {(hit) => (
                  <li class="flex gap-base border-b border-g1 py-snug">
                    <span class="w-figure shrink-0 text-text-disabled">{hit.day}</span>
                    <span class="w-figure shrink-0 text-text-faint">{hit.kind}</span>
                    <a href={toFragment({ kind: "building", address: hit.building })} class="shrink-0 text-text-quiet">
                      {hit.building}
                    </a>
                    <span class="flex-1 truncate text-text">{hit.subject}</span>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        )}
      </Show>
    </div>
  );
}

function Bin() {
  const ui = useUi();
  const say = useSay();
  const lang = useLang();
  const discards = ui.conn.asking.ask("discard_view");
  const rows = createMemo(() => {
    const held = discards();
    return held !== undefined && "discards" in held ? held.discards.rows : undefined;
  });
  const way = (row: { restoration?: { tracked: string } | { interred: string } | { rebuildable: { reason: string } } | null | undefined }) => {
    const r = row.restoration;
    if (r === null || r === undefined) return "";
    if ("tracked" in r) return `${say("bin_tracked")} ${r.tracked}`;
    if ("interred" in r) return `${say("bin_interred")} ${r.interred}`;
    return `${say("bin_rebuildable")} ${r.rebuildable.reason}`;
  };
  return (
    <Show when={rows()} fallback={<p class="text-text-disabled">…</p>}>
      {(held) => (
        <Show when={held().length > 0} fallback={<p class="text-text-faint">{say("bin_empty")}</p>}>
          <ul class="text-note">
            <For each={held()}>
              {(row) => (
                <li class="border-b border-g1 py-snug">
                  <div class="flex items-center gap-base">
                    <span class="min-w-0 flex-1">
                      <Path path={row.path} />
                    </span>
                    <span class="text-text-faint">{clock(lang(), row.at)}</span>
                    <span class={row.restored ? "text-text-disabled" : "text-alert"}>{row.restored ? say("bin_restored") : say("bin_gone")}</span>
                  </div>
                  <div class="mt-tight truncate font-mono text-text-faint">{way(row)}</div>
                </li>
              )}
            </For>
          </ul>
        </Show>
      )}
    </Show>
  );
}

// The five levels `docs/logging.md` names, in the order it names them.
const LEVELS = ["refuse", "effect", "decide", "trace", "wire"] as const;

function Log() {
  const say = useSay();
  return (
    <div class="flex flex-col gap-base">
      <Notice seat="entry" title={say("log_absent")} detail={say("log_absent_why")} />
      <div class="flex flex-wrap items-center gap-tight" role="group" aria-label={say("log_levels")}>
        <For each={LEVELS}>{(level) => <Button label={say(`log_${level}`)} tone="quiet" why={say("log_absent_why")} />}</For>
      </div>
      <EmptyState text={say("log_empty")} />
    </div>
  );
}

// The log lens is not in the address bar: `core/route` spells three
// lenses, and a fourth spelling is a change to the one translation
// between a view and the address bar. Until it is spelled there, the
// log is reached by the tab and not by a bookmark.
const LOG = "log";

export function Record(props: { readonly lens: Lens }) {
  const say = useSay();
  const go = useGo();
  const [log, setLog] = createSignal(false);
  const current = () => (log() ? LOG : props.lens);
  const lenses = () => [
    ...LENSES.map((lens) => ({ id: lens, label: say(`rec_${lens}`) })),
    { id: LOG, label: say("rec_log") },
  ];
  const pick = (id: string) => {
    const lens = LENSES.find((each) => each === id);
    if (lens === undefined) {
      setLog(true);
      return;
    }
    setLog(false);
    go({ kind: "record", lens });
  };
  return (
    <div class="mx-auto w-full max-w-page px-pane py-wide">
      <div class="mb-wide flex flex-wrap items-baseline gap-wide">
        <h1 class="text-title font-title">{say("nav_the_record")}</h1>
        <Tabs label={say("rec_lenses")} lenses={lenses()} current={current()} onPick={pick} />
      </div>
      <Switch>
        <Match when={current() === LOG}>
          <Log />
        </Match>
        <Match when={current() === "ledger"}>
          <Ledger />
        </Match>
        <Match when={current() === "archive"}>
          <Archive />
        </Match>
        <Match when={current() === "bin"}>
          <Bin />
        </Match>
      </Switch>
    </div>
  );
}
