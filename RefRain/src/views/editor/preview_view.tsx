// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, Match, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import type { Positions } from "../../core/document_pos";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, Encoding, Laid as Window } from "../../wire";
import { openDocument } from "../inspect/open";
import Empty from "../parts/empty.solid";
import Unanswered from "../parts/unanswered.solid";
import Laid from "./laid.solid";
import { short } from "./reading";

export default function Preview(props: {
  readonly positions: Positions;
  readonly building: Address;
  readonly at: Address;
  readonly drafted: boolean;
  readonly anchor: number | null;
  readonly onTop: (byte: number) => void;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const [windows, setWindows] = createSignal<readonly Window[]>([]);
  const [unsupported, setUnsupported] = createSignal<Encoding>();
  const [lost, setLost] = createSignal<string>();
  const [near, setNear] = createSignal(true);
  const [scroller, setScroller] = createSignal<HTMLDivElement>();
  const [sentinel, setSentinel] = createSignal<HTMLDivElement>();
  const total = createMemo(() => props.positions.bytes(props.positions.editor.length));
  const reached = createMemo(() => windows().at(-1)?.span.end ?? 0);
  const done = createMemo(() => windows().length > 0 && reached() >= total());
  const question = createMemo(() => ({
    preview: { version: props.positions.version, viewport: { start: reached(), end: total() } },
  }));
  const waiting = createMemo(
    () =>
      !done() &&
      unsupported() === undefined &&
      lost() === undefined &&
      (near() || (props.anchor !== null && props.anchor >= reached())),
  );
  const answer = storeValue(() => (waiting() ? u.conn.asking.ask(question()) : undefined));
  createEffect(
    () => ({ answer: answer(), from: reached() }),
    ({ answer, from }) => {
      const read = readAnswer(answer, (held) => ("preview" in held ? held.preview : undefined));
      if (read.kind === "unavailable") setLost(read.query);
      if (read.kind !== "held") return;
      const preview = read.value.preview;
      if ("unsupported" in preview) setUnsupported(preview.unsupported.encoding);
      else if (preview.laid.span.start === from && (windows().at(-1)?.span.end ?? 0) === from) {
        setWindows((current) => [...current, preview.laid]);
      }
    },
  );
  createEffect(
    () => ({ at: sentinel(), root: scroller() }),
    ({ at, root }) => {
      if (at === undefined) return;
      const seen = new IntersectionObserver(
        (entries) => setNear(entries.some((entry) => entry.isIntersecting)),
        {
          root: root ?? null,
          rootMargin: "600px",
        },
      );
      seen.observe(at);
      return () => seen.disconnect();
    },
  );
  function placed(): { readonly element: HTMLElement; readonly start: number }[] {
    return [...(scroller()?.querySelectorAll<HTMLElement>("[data-start]") ?? [])].map(
      (element) => ({
        element,
        start: Number(element.dataset.start ?? "0"),
      }),
    );
  }
  createEffect(
    () => ({ anchor: props.anchor, reached: reached(), done: done() }),
    ({ anchor, reached, done }) => {
      if (anchor === null || (anchor >= reached && !done)) return;
      const target = placed()
        .filter((each) => each.start <= anchor)
        .at(-1);
      target?.element.scrollIntoView({ block: "start" });
      props.onTop(target?.start ?? 0);
    },
  );
  function scrolled(): void {
    const edge = scroller()?.getBoundingClientRect().top ?? 0;
    const first = placed().find((each) => each.element.getBoundingClientRect().bottom > edge);
    if (first !== undefined) props.onTop(first.start);
  }
  function open(target: string): void {
    const path = target.split(/[?#]/u)[0] ?? "";
    const parts =
      `${props.at.slice(props.building.length + 1, props.at.lastIndexOf("/") + 1)}${path}`.split(
        "/",
      );
    const walked: string[] = [];
    for (const part of parts) {
      if (part === "..") {
        if (walked.pop() === undefined) return;
      } else if (part !== "." && part !== "") walked.push(part);
    }
    if (walked.length > 0)
      openDocument({ building: props.building, path: walked.join("/"), version: null });
  }
  return (
    <div
      class="min-h-0 flex-1 overflow-y-auto"
      ref={(node) => setScroller(node)}
      onScroll={scrolled}
    >
      {props.drafted ? (
        <p class="px-wide py-snug text-note text-text-faint">
          {fill(say(lang(), "refrain_preview_draft"), { version: short(props.positions.version) })}
        </p>
      ) : undefined}
      <Switch
        fallback={
          <div class="mx-auto w-full max-w-measure px-pane py-base">
            <For each={windows()} keyed={(window) => window.span.start}>
              {(window) => <Laid blocks={window().blocks} onOpen={open} placed />}
            </For>
            <div ref={(node) => setSentinel(node)} class="h-px" />
          </div>
        }
      >
        <Match when={unsupported()}>
          {(encoding) => (
            <p class="p-pane text-note text-text-quiet">
              {fill(say(lang(), "refrain_preview_unsupported"), { encoding: encoding() })}
            </p>
          )}
        </Match>
        <Match when={lost()}>
          {(query) => (
            <div class="p-pane">
              <Unanswered query={query()} asked={question()} />
            </div>
          )}
        </Match>
        <Match when={done() && windows().every((each) => each.blocks.length === 0)}>
          <Empty missing="refrain_preview_empty" />
        </Match>
      </Switch>
    </div>
  );
}
