// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Portal, type JSX } from "@solidjs/web";
import { For, Show, createEffect, createSignal, onCleanup, onSettled } from "solid-js";
import { matches, type Chord } from "../../core/keys";
import { say, type Lang } from "../../core/lang";
import { pressedOf } from "../../core/press";
import { ContextRow, opens, wanted } from "./context_tool_row";
import type { GlyphName } from "./glyph";
import Glyph from "./glyph.solid";
import "./context_tools.css";

export { attachContextTools, type ContextGestures } from "./context_gesture";

export interface ContextTrigger {
  readonly kind: "pointer" | "keyboard" | "touch";
  readonly event?: MouseEvent | KeyboardEvent | undefined;
  readonly x?: number | undefined;
  readonly y?: number | undefined;
}

export type ContextAnchor =
  | { readonly x: number; readonly y: number }
  | Pick<DOMRectReadOnly, "left" | "top" | "right" | "bottom">;

export interface ContextToolsControls {
  readonly back: () => void;
  readonly close: (restore?: boolean) => void;
}

export interface ContextTool {
  readonly id: string;
  readonly label: string;
  readonly glyph?: GlyphName;
  readonly shortcut?: Chord;
  readonly checked?: boolean | (() => boolean);
  readonly disabled?: boolean | (() => boolean);
  readonly children?: readonly ContextTool[];
  readonly panel?: (controls: ContextToolsControls) => JSX.Element;
  readonly hold?: (active: boolean) => void;
  readonly run?: () => void;
}

export interface ContextRequest {
  readonly tools: readonly ContextTool[];
  readonly label?: string | undefined;
  readonly anchor?: ContextAnchor | undefined;
  readonly restore?: HTMLElement | (() => void) | undefined;
}

interface Level {
  readonly tools: readonly ContextTool[];
  readonly label: string | undefined;
  readonly panel: ContextTool["panel"];
  readonly returnTo: string | undefined;
}

interface OpenMenu {
  readonly trigger: ContextTrigger;
  readonly request: ContextRequest;
  readonly mount: HTMLElement;
  readonly focus: string | undefined;
  readonly levels: readonly Level[];
}

export interface ContextToolsController extends ContextToolsControls {
  readonly state: () => OpenMenu | null;
  readonly holding: () => string | undefined;
  readonly open: (request: ContextRequest, trigger: ContextTrigger) => boolean;
  readonly enter: (tool: ContextTool) => void;
  readonly hold: (tool: ContextTool | undefined) => void;
}

const MAX_LEVELS = 3;
let active: ContextToolsController | undefined;

export function backContextTools(): boolean {
  if (active === undefined) return false;
  active.back();
  return true;
}

function openerOf(event: ContextTrigger["event"]): HTMLElement | undefined {
  const target = event?.target;
  if (target instanceof Element) {
    const focus = target.closest<HTMLElement>(
      "button, input, textarea, a[href], [tabindex], [contenteditable=true]",
    );
    if (focus instanceof HTMLElement) return focus;
  }
  return document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
}

export function createContextTools(): ContextToolsController {
  const [state, setState] = createSignal<OpenMenu | null>(null);
  const [holding, setHolding] = createSignal<string>();
  let showing: OpenMenu | null = null;
  let held: ContextTool | undefined;
  const publish = (menu: OpenMenu | null): void => {
    showing = menu;
    setState(menu);
  };
  const hold = (tool: ContextTool | undefined): void => {
    if (held === tool) return;
    held?.hold?.(false);
    held = tool;
    setHolding(tool?.id);
    held?.hold?.(true);
  };

  const controller: ContextToolsController = {
    state,
    holding,
    hold,
    open(request, trigger) {
      if (request.tools.length === 0) return false;
      active?.close(false);
      const target = openerOf(trigger.event);
      const restored = request.restore ?? target;
      const mount = target?.closest<HTMLElement>("dialog[open]") ?? document.body;
      active = controller;
      publish({
        trigger,
        mount,
        focus: undefined,
        request: { ...request, restore: restored },
        levels: [
          { tools: request.tools, panel: undefined, returnTo: undefined, label: request.label },
        ],
      });
      return true;
    },
    close(restore = true) {
      const target = showing?.request.restore;
      if (showing === null) return;
      hold(undefined);
      publish(null);
      if (active === controller) active = undefined;
      if (!restore) return;
      if (typeof target === "function") target();
      else if (target?.isConnected === true) target.focus({ preventScroll: true });
    },
    back() {
      if (showing === null) return;
      if (showing.levels.length === 1) {
        controller.close();
        return;
      }
      hold(undefined);
      const layer = showing.levels.at(-1);
      publish({ ...showing, levels: showing.levels.slice(0, -1), focus: layer?.returnTo });
    },
    enter(tool) {
      if (showing === null || wanted(tool.disabled)) return;
      if (opens(tool)) {
        if (showing.levels.length >= MAX_LEVELS) return;
        hold(undefined);
        publish({
          ...showing,
          focus: undefined,
          levels: [
            ...showing.levels,
            {
              tools: tool.children ?? [],
              panel: tool.panel,
              returnTo: tool.id,
              label: tool.label,
            },
          ],
        });
      } else if (tool.hold === undefined && tool.run !== undefined) {
        controller.close();
        tool.run();
      }
    },
  };

  onCleanup(() => {
    controller.close(false);
  });
  return controller;
}

interface Props {
  readonly controller: ContextToolsController;
  readonly lang: Lang;
}

function point(menu: OpenMenu): { x: number; y: number; above: number } {
  const anchor = menu.request.anchor;
  if (anchor !== undefined) {
    if ("left" in anchor) return { x: anchor.left, y: anchor.bottom, above: anchor.top };
    return { ...anchor, above: anchor.y };
  }
  const x = menu.trigger.x ?? 0;
  const y = menu.trigger.y ?? 0;
  return { x, y, above: y };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(min, max), Math.max(min, value));
}

function place(node: HTMLElement, menu: OpenMenu): void {
  const view = window.visualViewport;
  const left = view?.offsetLeft ?? 0;
  const top = view?.offsetTop ?? 0;
  const width = view?.width ?? window.innerWidth;
  const height = view?.height ?? window.innerHeight;
  const css = window.getComputedStyle(node);
  const inset = Number.parseFloat(css.paddingInlineStart) || 8;
  const touch = menu.trigger.kind === "touch";
  const shelf = touch ? 56 : 0;
  node.style.setProperty("--context-width", `${String(Math.max(0, width - 2 * inset))}px`);
  node.style.setProperty(
    "--context-height",
    `${String(Math.max(0, height - shelf - 2 * inset))}px`,
  );
  const rect = node.getBoundingClientRect();
  const at = point(menu);
  const right = left + width - inset;
  const bottom = top + height - inset - shelf;
  const x = touch ? left + inset : clamp(at.x, left + inset, right - rect.width);
  const preferred =
    at.y + inset + rect.height <= bottom ? at.y + inset : at.above - inset - rect.height;
  const y = touch
    ? Math.max(top + inset, bottom - rect.height)
    : clamp(preferred, top + inset, bottom - rect.height);
  node.style.setProperty("--context-x", `${String(x)}px`);
  node.style.setProperty("--context-y", `${String(y)}px`);
  node.dataset.positioned = "true";
}

function toolRows(deck: HTMLElement): HTMLButtonElement[] {
  return Array.from(deck.querySelectorAll<HTMLButtonElement>("button[data-context-item]"));
}

function chooseRow(deck: HTMLElement, id: string | undefined): void {
  const rows = toolRows(deck);
  const row =
    rows.find((item) => id !== undefined && item.dataset.contextItem === id) ??
    rows.find((item) => item.dataset.contextBack === undefined) ??
    rows[0];
  const target = row ?? deck;
  target.focus({ preventScroll: true });
}

function ContextSurface(props: Props) {
  const refs: { deck?: HTMLDivElement } = {};
  let prefix = "";
  let typedAt = 0;
  const menu = () => props.controller.state();
  const level = () => menu()?.levels.at(-1);
  const depth = () => menu()?.levels.length ?? 0;

  function keys(event: KeyboardEvent): void {
    const { deck } = refs;
    if (deck === undefined) return;
    const rows = toolRows(deck);
    const current = document.activeElement;
    const index = rows.findIndex((row) => row === current);
    if (event.key === "Tab" && level()?.panel === undefined) {
      event.stopPropagation();
      props.controller.close();
      return;
    }
    if (event.key === "ArrowLeft" && index >= 0 && depth() > 1) {
      event.preventDefault();
      event.stopPropagation();
      props.controller.back();
      return;
    }
    if (level()?.panel !== undefined) return;
    const shortcut = event.isComposing
      ? undefined
      : level()?.tools.find(
          (tool) =>
            tool.shortcut !== undefined &&
            !wanted(tool.disabled) &&
            matches(tool.shortcut, pressedOf(event)),
        );
    if (shortcut !== undefined) {
      event.preventDefault();
      event.stopPropagation();
      props.controller.enter(shortcut);
      return;
    }
    if (
      event.key === "ArrowRight" &&
      index >= 0 &&
      current?.getAttribute("aria-haspopup") === "menu"
    ) {
      event.preventDefault();
      event.stopPropagation();
      rows[index]?.click();
      return;
    }
    const last = rows.length - 1;
    if (last < 0) return;
    const positions: Readonly<Record<string, number>> = {
      ArrowDown: index === last ? 0 : index + 1,
      ArrowUp: index <= 0 ? last : index - 1,
      Home: 0,
      End: last,
    };
    const aimed = positions[event.key];
    if (aimed !== undefined) {
      event.preventDefault();
      event.stopPropagation();
      rows[aimed]?.focus();
    } else if (
      event.key.length === 1 &&
      event.key !== " " &&
      !event.altKey &&
      !event.ctrlKey &&
      !event.metaKey
    ) {
      const now = performance.now();
      if (now - typedAt > 500) prefix = "";
      typedAt = now;
      prefix += event.key.toLocaleLowerCase(props.lang);
      const ordered = [...rows.slice(index + 1), ...rows.slice(0, index + 1)];
      const match = ordered.find((row) =>
        (row.textContent ?? "").toLocaleLowerCase(props.lang).trim().startsWith(prefix),
      );
      if (match !== undefined) {
        event.preventDefault();
        event.stopPropagation();
        match.focus();
      }
    }
  }

  onSettled(() => {
    const { deck } = refs;
    if (deck === undefined) return;
    const controller = new AbortController();
    const signal = controller.signal;
    const position = (): void => {
      const shown = menu();
      if (shown !== null) place(deck, shown);
    };
    const observer = new ResizeObserver(position);
    observer.observe(deck);
    window.addEventListener("resize", position, { signal });
    window.visualViewport?.addEventListener("resize", position, { signal });
    window.visualViewport?.addEventListener("scroll", position, { signal });
    document.addEventListener(
      "pointerdown",
      (event) => {
        if (event.target instanceof Node && !deck.contains(event.target))
          props.controller.close(false);
      },
      { capture: true, signal },
    );
    document.addEventListener(
      "keydown",
      (event) => {
        if (event.key !== "Escape" || event.defaultPrevented || active !== props.controller) return;
        event.preventDefault();
        event.stopPropagation();
        props.controller.back();
      },
      { capture: true, signal },
    );
    window.addEventListener(
      "blur",
      () => {
        props.controller.close(false);
      },
      { signal },
    );
    return () => {
      controller.abort();
      observer.disconnect();
    };
  });

  createEffect(menu, (shown) => {
    const { deck } = refs;
    if (shown === null || deck === undefined) return;
    const frame = requestAnimationFrame(() => {
      place(deck, shown);
      chooseRow(deck, shown.focus);
      prefix = "";
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  });

  return (
    <div
      class="context-tools"
      data-context-tools
      data-input={menu()?.trigger.kind}
      ref={(node) => {
        refs.deck = node;
      }}
      tabindex={-1}
      onKeyDown={keys}
    >
      <Show
        when={level()?.panel}
        fallback={
          <div role="menu" aria-label={level()?.label ?? say(props.lang, "context_tools_title")}>
            <Show when={depth() > 1}>
              <button
                type="button"
                role="menuitem"
                tabindex={-1}
                data-context-item=""
                data-context-back=""
                data-control="surface"
                onClick={props.controller.back}
              >
                <Glyph name="go-back" size="sm" />
                {say(props.lang, "shell_go_back")}
              </button>
            </Show>
            <For each={level()?.tools}>
              {(tool) => (
                <ContextRow
                  tool={tool}
                  disabled={wanted(tool.disabled) || (opens(tool) && depth() >= MAX_LEVELS)}
                  controller={props.controller}
                />
              )}
            </For>
          </div>
        }
      >
        {(panel) => (
          <section
            role="dialog"
            aria-label={level()?.label ?? say(props.lang, "context_tools_title")}
          >
            <button
              type="button"
              data-context-item=""
              data-context-back=""
              data-control="surface"
              onClick={props.controller.back}
            >
              <Glyph name="go-back" size="sm" />
              {say(props.lang, "shell_go_back")}
            </button>
            {panel()(props.controller)}
          </section>
        )}
      </Show>
    </div>
  );
}

export function ContextTools(props: Props) {
  return (
    <Show when={props.controller.state()}>
      {(shown) => (
        <Portal mount={shown().mount}>
          <ContextSurface {...props} />
        </Portal>
      )}
    </Show>
  );
}
