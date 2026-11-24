// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { HOLD_MS } from "../../core/press";
import type { ContextRequest, ContextToolsController, ContextTrigger } from "./context_tools";

type FindTools = (trigger: ContextTrigger) => ContextRequest | null | undefined;

export interface ContextGestures {
  readonly longPress?: boolean | ((event: PointerEvent) => boolean);
}

const MOVE_LIMIT = 10;
const CLICK_WINDOW = 400;
const claimed = new WeakSet<Event>();

function near(x: number, y: number, origin: PointerEvent): boolean {
  return Math.hypot(x - origin.clientX, y - origin.clientY) <= MOVE_LIMIT;
}

export function attachContextTools(
  element: HTMLElement,
  controller: ContextToolsController,
  find: FindTools,
  options: ContextGestures = {},
): () => void {
  const listeners = new AbortController();
  const settings = { signal: listeners.signal };
  const doc = element.ownerDocument;
  let press: AbortController | undefined;
  let delayed: ReturnType<typeof setTimeout> | undefined;
  let clickGuard: AbortController | undefined;
  let clickExpiry: ReturnType<typeof setTimeout> | undefined;
  let suppress: PointerEvent | undefined;

  function clear(): void {
    clearTimeout(delayed);
    press?.abort();
    press = undefined;
  }

  function open(trigger: ContextTrigger): boolean {
    if (trigger.event !== undefined && claimed.has(trigger.event)) return false;
    const request = find(trigger);
    if (request == null || !controller.open(request, trigger)) return false;
    if (trigger.event !== undefined) claimed.add(trigger.event);
    return true;
  }

  function consume(event: Event): void {
    event.preventDefault();
    event.stopPropagation();
  }

  function clearClick(): void {
    clearTimeout(clickExpiry);
    clickGuard?.abort();
    clickGuard = undefined;
    suppress = undefined;
  }

  function expireClick(): void {
    clearTimeout(clickExpiry);
    clickExpiry = setTimeout(clearClick, CLICK_WINDOW);
  }

  function guardClick(origin: PointerEvent): void {
    clearClick();
    const guard = new AbortController();
    clickGuard = guard;
    suppress = origin;
    doc.addEventListener(
      "click",
      (event) => {
        if (
          event.detail === 0 ||
          !near(event.clientX, event.clientY, origin) ||
          (event instanceof PointerEvent &&
            event.pointerType !== "" &&
            event.pointerType !== "touch")
        )
          return;
        clearClick();
        consume(event);
        event.stopImmediatePropagation();
      },
      { capture: true, signal: guard.signal },
    );
  }

  element.addEventListener(
    "keydown",
    (event) => {
      if (
        event.defaultPrevented ||
        event.repeat ||
        !(event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey))
      )
        return;
      const target = event.target;
      const rect = target instanceof Element ? target.getBoundingClientRect() : undefined;
      if (open({ kind: "keyboard", event, x: rect?.left, y: rect?.bottom })) consume(event);
    },
    settings,
  );

  element.addEventListener(
    "contextmenu",
    (event) => {
      if (event.defaultPrevented) return;
      if (suppress !== undefined && near(event.clientX, event.clientY, suppress)) {
        consume(event);
        return;
      }
      const touch = event instanceof PointerEvent && event.pointerType === "touch";
      if (
        touch &&
        (options.longPress === false ||
          (typeof options.longPress === "function" && !options.longPress(event)))
      )
        return;
      if (open({ kind: touch ? "touch" : "pointer", event, x: event.clientX, y: event.clientY })) {
        if (touch && event instanceof PointerEvent) {
          guardClick(event);
          expireClick();
          clear();
        }
        consume(event);
      }
    },
    settings,
  );

  element.addEventListener(
    "pointerdown",
    (event) => {
      if (
        event.defaultPrevented ||
        event.pointerType !== "touch" ||
        !event.isPrimary ||
        event.button !== 0 ||
        options.longPress === false ||
        (typeof options.longPress === "function" && !options.longPress(event))
      )
        return;
      clear();
      press = new AbortController();
      const tracking = { capture: true, passive: true, signal: press.signal };
      const id = event.pointerId;
      const cancel = (): void => {
        if (suppress === event) {
          controller.close(false);
          clearClick();
        }
        clear();
      };
      delayed = setTimeout(() => {
        if (open({ kind: "touch", event, x: event.clientX, y: event.clientY })) {
          guardClick(event);
        }
      }, HOLD_MS);
      doc.addEventListener(
        "pointermove",
        (move) => {
          if (move.pointerId === id && !near(move.clientX, move.clientY, event)) cancel();
        },
        tracking,
      );
      doc.addEventListener(
        "pointerdown",
        (other) => {
          if (other.pointerId !== id) cancel();
        },
        tracking,
      );
      doc.addEventListener(
        "pointerup",
        (up) => {
          if (up.pointerId !== id) return;
          if (suppress === event) expireClick();
          clear();
        },
        tracking,
      );
      doc.addEventListener(
        "pointercancel",
        (canceled) => {
          if (canceled.pointerId === id) cancel();
        },
        tracking,
      );
      doc.addEventListener("scroll", cancel, tracking);
      doc.defaultView?.addEventListener("blur", cancel, { signal: press.signal });
    },
    { passive: true, signal: listeners.signal },
  );

  return () => {
    clear();
    clearClick();
    listeners.abort();
  };
}
