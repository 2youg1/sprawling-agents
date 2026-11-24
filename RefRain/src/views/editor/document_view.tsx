// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import {
  createEffect,
  createMemo,
  createSignal,
  For,
  lazy,
  Loading,
  Match,
  onCleanup,
  Show,
  Switch,
  untrack,
} from "solid-js";
import { draftPlace } from "../../core/document_save";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { kib } from "../../core/time";
import { ui } from "../../ui";
import type { Address, B3Hash, ProposalCard } from "../../wire";
import Empty from "../parts/empty.solid";
import Unanswered from "../parts/unanswered.solid";
import Unkept from "../parts/unkept_view";
import { carried } from "./carry";
import Conflict from "./conflict.solid";
import { drawnAs } from "./formats/format";
import Html from "./formats/html.solid";
import Opaque from "./formats/opaque_view";
import Head from "./head.solid";
import Line from "./line.look.solid";
import Preview from "./preview_view";
import Proposals from "./proposals_view";
import { phrasesIn, short, type Reading } from "./reading";
import { documentAt, Session, type Locked } from "./session";
import Versions from "./versions.solid";

const Source = lazy(() => import("./source_view"));

function exposesEditor(reading: Reading): boolean {
  return reading === "source" || reading === "live" || reading === "diff";
}

export function DocumentPage(props: {
  readonly at: Address;
  readonly building: Address;
  readonly version: B3Hash | null;
  readonly reading?: Reading;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const doc = untrack(() => new Session(u, props.at, props.version));
  const state = storeValue(doc.state);
  onCleanup(() => doc.close());
  const unkept = storeValue(() => u.prefs.draftUnkept(draftPlace(props.at)));
  const drafted = createMemo(
    () => state().receipt.kind !== "clean" && state().receipt.kind !== "saved",
  );
  const [reading, setReading] = createSignal<Reading>(untrack(() => props.reading ?? "source"));
  const [anchor, setAnchor] = createSignal<number | null>(null);
  let top = 0;
  const name = () => props.at.slice(props.at.lastIndexOf("/") + 1);
  const markdown = createMemo(() => {
    const file = state().file;
    return file?.kind === "text" && file.gathering.format === "markdown";
  });
  const html = createMemo(() => state().file?.kind === "text" && drawnAs(props.at) === "html");
  const phrases = createMemo(() => phrasesIn(lang()));
  const editorShown = createMemo(() => exposesEditor(reading()));
  createEffect(
    () => state().editing,
    (editing) => {
      editing?.setMarkdown(reading() === "live" ? "live" : "source");
      editing?.diffing(reading() === "diff");
    },
  );
  function pick(next: Reading): void {
    const { editing, positions: base } = state();
    let scrollTo: number | null = null;
    if (editing !== null && base !== null && markdown()) {
      const carry = carried(reading(), next, { top: editing.toBaseline(editing.top()) }, top);
      switch (carry.kind) {
        case "preview_at":
          setAnchor(base.bytes(carry.offset));
          break;
        case "editor_at":
          scrollTo = editing.fromBaseline(base.editorAt(carry.byte));
          break;
        case "nothing":
          break;
      }
    }
    editing?.diffing(next === "diff");
    editing?.setMarkdown(next === "live" ? "live" : "source");
    setReading(next);
    if (exposesEditor(next))
      queueMicrotask(() => {
        editing?.measure();
        if (scrollTo !== null) editing?.scrollTop(scrollTo);
      });
  }
  function lockedLine(locked: Locked): string {
    switch (locked.kind) {
      case "gathering":
        return fill(say(lang(), "refrain_gathering"), {
          through: kib(locked.through),
          total: kib(locked.total),
        });
      case "too_large":
        return fill(say(lang(), "refrain_too_large"), {
          through: kib(locked.through),
          total: kib(locked.total),
        });
      case "recorded":
        return fill(say(lang(), "refrain_recorded"), { version: short(locked.version) });
      case "lost":
        return fill(say(lang(), "refrain_lost"), { version: short(locked.version) });
    }
  }
  function unshowable(card: ProposalCard): string | undefined {
    const { editing, positions } = state();
    return editing !== null && positions?.version === card.baseline
      ? undefined
      : say(lang(), "proposal_show_why");
  }
  function show(card: ProposalCard): void {
    const { editing, positions } = state();
    if (editing === null || positions === null) return;
    if (!editorShown()) pick("source");
    editing.reveal(editing.fromBaseline(positions.editorAt(card.span.start)));
    editing.focus();
  }
  const refused = createMemo(() => {
    const receipt = state().receipt;
    return receipt.kind === "refused" ? receipt.error.recovery : undefined;
  });
  const missing = createMemo(() => {
    const read = state().read;
    return read.kind === "unavailable" ? read.query : undefined;
  });
  const unreadable = createMemo(() => {
    const file = state().file;
    return file?.kind === "unreadable" ? file : undefined;
  });
  const opaque = createMemo(() => {
    const file = state().file;
    return file?.kind === "opaque" ? file : undefined;
  });
  return (
    <div class="refrain flex min-h-0 flex-1 flex-col bg-page">
      <Head
        at={props.at}
        session={doc}
        reading={reading()}
        live={markdown()}
        previewed={markdown() || html()}
        onPick={pick}
      />
      <Show when={state().file?.kind === "text" ? state().locked : null}>
        {(locked) => <Line tone="faint">{lockedLine(locked())}</Line>}
      </Show>
      <Show when={refused()}>{(words) => <Line tone="alert">{words()}</Line>}</Show>
      <Show when={state().receipt.kind === "pending"}>
        <Line tone="faint">{say(lang(), "refrain_receipt_pending_why")}</Line>
      </Show>
      <Show when={unkept() && drafted()}>
        <Line tone="alert">
          <Unkept words={() => state().editing?.text() ?? ""} />
        </Line>
      </Show>
      <Show when={state().lostDraft}>
        {(version) => (
          <Line tone="alert">
            {fill(say(lang(), "refrain_draft_lost"), { version: short(version()) })}
          </Line>
        )}
      </Show>
      <Show when={reading() === "diff" && !drafted() ? state().positions : null}>
        {(positions) => (
          <Line tone="faint">
            {fill(say(lang(), "refrain_diff_same"), { version: short(positions().version) })}
          </Line>
        )}
      </Show>
      <Show when={state().receipt.kind === "conflict"}>
        <div class="px-wide py-snug">
          <Conflict name={name()} session={doc} onCompare={() => pick("versions")} />
        </div>
      </Show>
      <Proposals doc={props.at} unshowable={unshowable} onShow={show} />
      <Switch
        fallback={
          <>
            <div class={["min-h-0 flex-1 flex-col", editorShown() ? "flex" : "hidden"]}>
              <Loading fallback={null}>
                <Source
                  session={doc}
                  label={name()}
                  phrases={phrases()}
                  diagramLabel={markdown() ? say(lang(), "refrain_diagram") : undefined}
                />
              </Loading>
            </div>
            <Show when={reading() === "preview" ? state().positions : null}>
              {(positions) => (
                <Show
                  when={html()}
                  fallback={
                    <For each={[positions()]} keyed={(position) => position.version}>
                      {(baseline) => (
                        <Preview
                          positions={baseline()}
                          building={props.building}
                          at={props.at}
                          drafted={drafted()}
                          anchor={anchor()}
                          onTop={(byte) => {
                            top = byte;
                            setAnchor(null);
                          }}
                        />
                      )}
                    </For>
                  }
                >
                  <Html
                    text={state().editing?.text() ?? positions().editor}
                    drafted={drafted()}
                    name={name()}
                  />
                </Show>
              )}
            </Show>
            <Show when={reading() === "versions"}>
              <Versions at={props.at} session={doc} label={name()} phrases={phrases()} />
            </Show>
          </>
        }
      >
        <Match when={state().read.kind === "asking"}>
          <p class="p-pane text-text-faint">…</p>
        </Match>
        <Match when={missing()}>
          {(query) => (
            <div class="p-pane">
              <Unanswered query={query()} asked={{ document: { at: props.at } }} />
            </div>
          )}
        </Match>
        <Match when={state().file?.kind === "missing"}>
          <Empty missing="file_missing" />
        </Match>
        <Match when={unreadable()}>
          {(file) => (
            <p class="p-pane text-note text-text-quiet">
              {fill(say(lang(), "refrain_unreadable"), { reason: file().reason })}
            </p>
          )}
        </Match>
        <Match when={opaque()}>
          {(file) => <Opaque path={props.at} version={file().version} bytes={file().bytes} />}
        </Match>
        <Match when={state().opening === null}>
          <p class="p-pane text-text-faint">…</p>
        </Match>
      </Switch>
    </div>
  );
}

export default function RefRainDocument(props: {
  readonly building: Address;
  readonly path: string;
  readonly version: B3Hash | null;
}) {
  const lang = storeValue(ui().lang);
  const at = createMemo(() => documentAt(props.building, props.path));
  return (
    <Show
      when={at()}
      fallback={<p class="p-pane text-text-faint">{say(lang(), "file_missing")}</p>}
    >
      {(absolute) => (
        <For
          each={[{ at: absolute(), building: props.building, version: props.version }]}
          keyed={(item) => `${item.building}/${item.at}@${item.version ?? ""}`}
        >
          {(item) => (
            <DocumentPage at={item().at} building={item().building} version={item().version} />
          )}
        </For>
      )}
    </Show>
  );
}
