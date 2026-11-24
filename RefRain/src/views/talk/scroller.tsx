// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, createMemo, createSignal, Show, untrack } from "solid-js";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Glyph from "../parts/glyph.solid";
import { anchorAt, footOf, type Anchoring } from "./anchoring";
import { keepReading, readingAt } from "./standing";
import type { UnreadLook } from "./unread";
import Wear from "./wear_view";

interface ScrollerProps {
  readonly empty: boolean;
  readonly rejoined: number;
  readonly place: string;
  readonly children: JSX.Element;
}

export function UnreadKey(props: UnreadLook) {
  return (
    <button
      type={props.wire.type}
      data-control=""
      aria-label={props.wire["aria-label"]}
      class="inline-flex h-control-sm items-center gap-tight rounded-pill border border-edge-panel bg-raised px-base text-note text-text shadow-float hover:bg-raised-hover"
      onClick={() => props.wire.onclick()}
    >
      <Glyph name="chevron" size="sm" class="rotate-90" />
      {props.text}
    </button>
  );
}

export default function Scroller(props: ScrollerProps) {
  const lang = storeValue(ui().lang);
  const [box, setBox] = createSignal<HTMLDivElement>();
  const [column, setColumn] = createSignal<HTMLDivElement>();
  const [anchor, setAnchor] = createSignal<Anchoring>("follow");
  const [blocks, setBlocks] = createSignal(0);
  const [left, setLeft] = createSignal<number | null>(null);
  const unread = createMemo(() => {
    const seen = left();
    return seen === null ? 0 : Math.max(0, blocks() - seen);
  });
  const unreadWord = createMemo(() => fill(say(lang(), "talk_unread"), { n: String(unread()) }));
  let pending: number | null = null;

  function toFoot(): void {
    const view = box();
    if (view !== undefined) view.scrollTop = view.scrollHeight;
  }

  function follow(): void {
    setAnchor("follow");
    toFoot();
  }

  function reach(): void {
    const view = box();
    if (pending === null || view === undefined) return;
    view.scrollTop = pending;
    if (view.scrollTop >= pending - 1) pending = null;
  }

  function letGo(): void {
    pending = null;
  }

  function arrive(at: string): void {
    const top = readingAt(at);
    if (top === null) {
      letGo();
      setAnchor(anchorAt({ kind: "opened" }));
      toFoot();
      return;
    }
    setAnchor("hold");
    pending = top;
    reach();
  }

  let standing = untrack(() => props.place);
  createEffect(
    () => props.place,
    (place) => {
      if (standing === place) return;
      standing = place;
      arrive(place);
    },
  );
  let answered = untrack(() => props.rejoined);
  createEffect(
    () => props.rejoined,
    (rejoined) => {
      if (answered === rejoined) return;
      answered = rejoined;
      follow();
    },
  );
  createEffect(
    () => anchor(),
    (state) => {
      if (state === "follow") setLeft(null);
      else setLeft((previous) => previous ?? blocks());
    },
  );

  const observed = createMemo(() => {
    const head = box();
    const body = column();
    return head === undefined || body === undefined ? undefined : { head, body };
  });
  createEffect(observed, (nodes) => {
    if (nodes === undefined) return;
    const { head, body } = nodes;
    arrive(props.place);
    const measure = (): void => {
      setBlocks(body.querySelectorAll("[data-wear]").length);
    };
    measure();
    const watcher = new ResizeObserver(() => {
      measure();
      reach();
      const selection = document.getSelection();
      const selecting =
        selection !== null && !selection.isCollapsed && body.contains(selection.anchorNode);
      if (anchor() === "follow" && !selecting) toFoot();
    });
    watcher.observe(body);
    const hands = ["wheel", "pointerdown", "keydown"] as const;
    for (const hand of hands) head.addEventListener(hand, letGo, { passive: true });
    return () => {
      watcher.disconnect();
      for (const hand of hands) head.removeEventListener(hand, letGo);
    };
  });

  return (
    <div class="relative min-h-0 flex-1 -mx-wide narrow:mx-0">
      <div
        ref={(node) => {
          setBox(node);
        }}
        data-thread=""
        tabindex={-1}
        class={[
          "h-full overflow-y-auto px-wide [scrollbar-width:none] narrow:px-0 narrow:[scrollbar-width:auto] [mask-image:linear-gradient(to_bottom,transparent_0,black_160px)] transition-opacity duration-panel",
          props.empty ? "opacity-0 ease-leave" : "ease-arrive",
        ]}
        onScroll={(event) => {
          if (pending !== null) return;
          const next = anchorAt({ kind: "scrolled", foot: footOf(event.currentTarget) });
          setAnchor(next);
          keepReading(props.place, next === "hold" ? event.currentTarget.scrollTop : null);
        }}
      >
        <div
          ref={(node) => {
            setColumn(node);
          }}
          class="flex min-h-full w-full flex-col justify-end pt-section pb-section"
        >
          {props.children}
        </div>
      </div>
      <Show when={!props.empty && observed()}>
        {(nodes) => <Wear scroller={nodes().head} column={nodes().body} />}
      </Show>
      <Show when={unread() > 0}>
        <div class="absolute bottom-snug left-1/2 -translate-x-1/2">
          <UnreadKey
            text={unreadWord()}
            wire={{
              type: "button",
              "aria-label": `${unreadWord()} · ${say(lang(), "talk_unread_follow")}`,
              onclick: () => {
                follow();
                box()?.focus({ preventScroll: true });
              },
            }}
          />
        </div>
      </Show>
    </div>
  );
}
