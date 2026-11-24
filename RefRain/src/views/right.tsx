// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import {
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  For,
  Match,
  onSettled,
  Show,
  Switch,
} from "solid-js";
import { say, type Lang } from "../core/lang";
import { toFragment } from "../core/route";
import { storeValue } from "../core/solid_store";
import { derivedList } from "../core/store";
import { markEndAtFrame, markStart } from "../core/timing";
import { scopedUi, ui } from "../ui";
import type { Address, Call, RoundsAnswer } from "../wire";
import Changes from "./changes.solid";
import RefRain from "./editor/document_view";
import Called from "./inspect/called.solid";
import Letter from "./inspect/letter.solid";
import {
  createInspector,
  frontWhere,
  itemKey,
  openItems,
  rightItem,
  sameItem,
  sharedInspector,
  type RightItem,
} from "./inspect/open";
import { readFrom, readingOf, regionOf, type Region, type Tab } from "./inspect/reading";
import Split from "./inspect/split.solid";
import Strip from "./inspect/strip.solid";
import { commandOf } from "./monitor/trace";
import ContextReport from "./talk/context_report_view";

export interface RightProps {
  readonly following: readonly RightItem[];
  readonly current: RoundsAnswer | undefined;
  readonly talk: Address;
  readonly inspection?: ReturnType<typeof createInspector>;
}

type ReadCall = { readonly rounds: RoundsAnswer; readonly call: Call | null };

const PANELS: Readonly<Record<Region, string>> = {
  editor: "inspector-editor",
  terminal: "inspector-terminal",
};
const LEAST = 6;
const SHORT = 7;

function ItemPage(props: {
  readonly item: RightItem;
  readonly talk: Address;
  readonly lang: Lang;
  readonly find: (item: RightItem) => ReadCall | null;
}) {
  return (
    <Switch
      fallback={
        <CallPage item={props.item} talk={props.talk} lang={props.lang} find={props.find} />
      }
    >
      <Match when={props.item.kind === "document" ? props.item : null}>
        {(item) => (
          <div class="min-h-0 flex-1 overflow-auto bg-page">
            <RefRain building={item().building} path={item().path} version={item().version} />
          </div>
        )}
      </Match>
      <Match when={props.item.kind === "letter" ? props.item : null}>
        {(item) => <Letter doc={item().doc} card={item().card} />}
      </Match>
      <Match when={props.item.kind === "changes" ? props.item : null}>
        {(item) => (
          <div class="min-h-0 flex-1 overflow-auto bg-page px-wide py-snug">
            <Changes base={item().base} head={item().head} talk={props.talk} />
          </div>
        )}
      </Match>
      <Match when={props.item.kind === "context" ? props.item : null}>
        {(item) => (
          <div class="min-h-0 flex-1 overflow-auto bg-page">
            <ContextReport run={item().run} room={item().room} />
          </div>
        )}
      </Match>
    </Switch>
  );
}

function CallPage(props: {
  readonly item: RightItem;
  readonly talk: Address;
  readonly lang: Lang;
  readonly find: (item: RightItem) => ReadCall | null;
}) {
  const found = createMemo(() => props.find(props.item));
  const absent = createMemo(() => (found() === null ? "…" : say(props.lang, "inspect_call_gone")));
  return (
    <Show
      when={found()?.call}
      fallback={<p class="px-wide py-snug text-note text-text-faint">{absent()}</p>}
    >
      {(call) => (
        <Show when={found()?.rounds}>
          {(rounds) => <Called call={call()} rounds={rounds()} talk={props.talk} />}
        </Show>
      )}
    </Show>
  );
}

export default function Right(props: RightProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const scope = scopedUi();
  const inspection =
    props.inspection ?? (scope === undefined ? sharedInspector : createInspector());
  const uid = createUniqueId();
  const panels =
    scope === undefined
      ? PANELS
      : { editor: `${uid}-${PANELS.editor}`, terminal: `${uid}-${PANELS.terminal}` };
  const state = storeValue(inspection.items);
  const held = createMemo(() => openItems(state()));
  const items = createMemo(() => (held().length > 0 ? held() : props.following));
  const runs = createMemo(() => [
    ...new Set(items().flatMap((item) => (item.kind === "call" ? [item.run] : []))),
  ]);
  const asked = storeValue(() =>
    derivedList(
      runs().map((run) => u.conn.asking.ask({ rounds: { run } })),
      (answers) =>
        new Map(
          answers.flatMap((answer) =>
            answer !== undefined && "rounds" in answer
              ? [[answer.rounds.run, answer.rounds] as const]
              : [],
          ),
        ),
    ),
  );
  const rounds = createMemo(() =>
    props.current === undefined || asked().has(props.current.run)
      ? asked()
      : new Map([...asked(), [props.current.run, props.current]]),
  );

  function callOf(item: RightItem): ReadCall | null {
    if (item.kind !== "call") return null;
    const answer = rounds().get(item.run);
    if (answer === undefined) return null;
    return {
      rounds: answer,
      call: answer.turns.flatMap((turn) => turn.calls).find((call) => call.at === item.at) ?? null,
    };
  }

  function regionOfItem(item: RightItem): Region {
    const call = callOf(item)?.call ?? null;
    return call === null ? "editor" : regionOf(readingOf(call));
  }

  function labelOf(item: RightItem): string {
    if (item.kind === "document") return item.path.slice(item.path.lastIndexOf("/") + 1);
    if (item.kind === "changes") return item.head.slice(0, SHORT);
    if (item.kind === "letter") return item.doc.slice(item.doc.lastIndexOf("/") + 1);
    if (item.kind === "context") return say(lang(), "context_report_tab");
    const call = callOf(item)?.call ?? null;
    if (call === null) return "…";
    const entry = call.render === "terminal" ? commandOf(call) : null;
    if (entry?.kind === "command") return entry.text;
    const path = readFrom(call)?.path ?? (readingOf(call) === "diff" ? (call.subject ?? "") : "");
    return path === "" ? call.tool : path.slice(path.lastIndexOf("/") + 1);
  }

  function linkOf(item: RightItem): string | null {
    switch (item.kind) {
      case "call":
        return toFragment({
          kind: "talk",
          address: props.talk,
          item: { kind: "call", run: item.run, at: item.at },
        });
      case "document":
        return toFragment({
          kind: "talk",
          address: props.talk,
          item: {
            kind: "document",
            building: item.building,
            path: item.path,
            version: item.version,
          },
        });
      case "changes":
      case "letter":
      case "context":
        return null;
    }
  }

  function frontOf(region: Region): RightItem | null {
    return held().length > 0
      ? frontWhere((item) => regionOfItem(item) === region, state())
      : (props.following.find((item) => regionOfItem(item) === region) ?? null);
  }

  const tabs = createMemo<readonly Tab[]>(() =>
    items().map((item) => ({ item, label: labelOf(item), region: regionOfItem(item) })),
  );
  const front = createMemo(() =>
    held().length > 0 ? rightItem(state()) : (props.following[0] ?? null),
  );
  const editorTabs = createMemo(() => tabs().filter((tab) => tab.region === "editor"));
  const terminalTabs = createMemo(() => tabs().filter((tab) => tab.region === "terminal"));
  const editorFront = createMemo(() => frontOf("editor"));
  const terminalFront = createMemo(() => frontOf("terminal"));

  function keepFollowing(item: RightItem): void {
    if (held().length === 0)
      for (const kept of props.following) if (!sameItem(item, kept)) inspection.showItem(kept);
  }
  function pick(item: RightItem): void {
    markStart("panel_frame");
    keepFollowing(item);
    inspection.showItem(item);
    markEndAtFrame("panel_frame");
  }
  function close(item: RightItem): void {
    keepFollowing(item);
    inspection.closeItem(item);
    if (held().length === 0) u.prefs.setPanel(false);
  }
  function closeAll(): void {
    inspection.closeRight();
    u.prefs.setPanel(false);
  }

  const [step, setStep] = createSignal(24);
  const [chosen, setChosen] = createSignal<number | null>(null);
  const [room, setRoom] = createSignal(0);
  const [editorHeight, setEditorHeight] = createSignal(0);
  const [area, setArea] = createSignal<HTMLDivElement>();
  const [editor, setEditor] = createSignal<HTMLDivElement>();
  const most = createMemo(() => Math.max(LEAST, Math.floor(room() / step()) - LEAST));
  const lines = createMemo(() => chosen() ?? Math.round(editorHeight() / step()));
  onSettled(() => {
    const baseline = Number.parseFloat(
      getComputedStyle(document.documentElement).getPropertyValue("--spacing-baseline"),
    );
    if (Number.isFinite(baseline) && baseline > 0) setStep(3 * baseline);
  });
  createEffect(area, (node) => watchHeight(node, setRoom));
  createEffect(editor, (node) => watchHeight(node, setEditorHeight));

  return (
    <aside
      data-inspector=""
      aria-label={say(lang(), "inspect_label")}
      class="side-in flex h-full min-h-0 w-full flex-col bg-chrome"
      onKeyDown={(event) => {
        if (event.key !== "Escape" || event.isComposing || event.defaultPrevented) return;
        event.preventDefault();
        closeAll();
      }}
    >
      <Strip
        tabs={tabs()}
        front={front()}
        panels={panels}
        linkOf={linkOf}
        onPick={pick}
        onClose={close}
        onCloseAll={closeAll}
      />
      <div
        class="side-in-then flex min-h-0 flex-1 flex-col"
        ref={(node) => {
          setArea(node);
        }}
      >
        <Show when={editorTabs().length > 0}>
          <div
            id={panels.editor}
            role="tabpanel"
            aria-label={say(lang(), "inspect_editor")}
            class={[
              "flex min-h-0 flex-col bg-page",
              chosen() === null && terminalTabs().length > 0 ? "flex-[3]" : "",
              terminalTabs().length === 0 ? "flex-1" : "shrink-0",
            ]}
            style={{ height: fixedHeight(chosen(), terminalTabs().length, step()) }}
            ref={(node) => {
              setEditor(node);
            }}
          >
            <For each={editorTabs()} keyed={(tab) => itemKey(tab.item)}>
              {(tab) => (
                <div
                  class={
                    sameItem(editorFront(), tab().item) ? "flex min-h-0 flex-1 flex-col" : "hidden"
                  }
                >
                  <ItemPage item={tab().item} talk={props.talk} lang={lang()} find={callOf} />
                </div>
              )}
            </For>
          </div>
        </Show>
        <Show when={editorTabs().length > 0 && terminalTabs().length > 0}>
          <Split
            lines={Math.min(most(), Math.max(LEAST, lines()))}
            least={LEAST}
            most={most()}
            controls={panels.editor}
            step={step()}
            onLines={setChosen}
            onReset={() => {
              setChosen(null);
            }}
          />
        </Show>
        <Show when={terminalTabs().length > 0}>
          <div
            id={panels.terminal}
            role="tabpanel"
            aria-label={say(lang(), "inspect_terminal")}
            class={[
              "flex min-h-0 flex-col",
              chosen() === null && editorTabs().length > 0 ? "flex-[2]" : "flex-1",
            ]}
          >
            <For each={terminalTabs()} keyed={(tab) => itemKey(tab.item)}>
              {(tab) => (
                <div
                  class={
                    sameItem(terminalFront(), tab().item)
                      ? "flex min-h-0 flex-1 flex-col"
                      : "hidden"
                  }
                >
                  <ItemPage item={tab().item} talk={props.talk} lang={lang()} find={callOf} />
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>
    </aside>
  );
}

function fixedHeight(lines: number | null, terminals: number, step: number): string | undefined {
  return lines === null || terminals === 0 ? undefined : `${String(lines * step)}px`;
}

function watchHeight(
  node: HTMLDivElement | undefined,
  read: (height: number) => void,
): (() => void) | undefined {
  if (node === undefined) return;
  const size = (): void => {
    read(node.clientHeight);
  };
  size();
  const observer = new ResizeObserver(size);
  observer.observe(node);
  return () => observer.disconnect();
}
