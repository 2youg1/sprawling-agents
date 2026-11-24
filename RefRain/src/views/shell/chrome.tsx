// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createMemo, createSignal, onCleanup, onSettled } from "solid-js";

import { say, type Lang } from "../../core/lang";
import { HOLD_MS } from "../../core/press";
import type { Subject } from "../../core/shell_visit";
import Glyph from "../parts/glyph.solid";
import { watchColumns } from "../shared/frame";

export const NAVIGATION_ID = "workspace-navigation";
export const CONVERSATION_ID = "workspace-conversation";
export const SETTINGS_KEY = "workspace-settings-key";
export const NAVIGATION_KEY = "workspace-navigation-key";
export const MAILBOX_KEY = "workspace-mailbox-key";
export const SUBJECT_KEY = "workspace-subject-key";
export const ACTIONS_KEY = "workspace-actions-key";
const ACTION_GROUP = "workspace-edge-action-group";

export interface ChromeProps {
  readonly scope?: string;
  readonly lang: Lang;
  readonly subject: Subject;
  readonly navigation: boolean;
  readonly settingsOpen: boolean;
  readonly onNavigation: (at: number) => void;
  readonly onSubject: () => void;
  readonly onSettings: (trigger: HTMLElement) => void;
  readonly mailbox?: JSX.Element;
}

let backForEdge: (() => boolean) | undefined;

export function backEdgeActions(): boolean {
  return backForEdge?.() ?? false;
}

export function scopedId(id: string, scope?: string): string {
  return scope === undefined ? id : `${scope}-${id}`;
}

export function focusEdgeKey(id: string, scope?: string): void {
  const key = document.getElementById(scopedId(id, scope));
  if (key?.getClientRects().length) key.focus({ preventScroll: true });
  else document.getElementById(scopedId(ACTIONS_KEY, scope))?.focus({ preventScroll: true });
}

export function Chrome(props: ChromeProps) {
  const id = (part: string) => scopedId(part, props.scope);
  const [narrow, setNarrow] = createSignal(window.matchMedia("(width < 768px)").matches);
  const [expanded, setExpanded] = createSignal(false);
  const compact = createMemo(() => narrow() && !expanded());
  const listName = () =>
    say(props.lang, props.subject === "files" ? "world_files" : "world_sessions");
  const measureInput =
    performance.getEntriesByName("sprawling:measure-shell-input", "mark").length > 0;
  let rail: HTMLDivElement | undefined;

  function activate(name: "disclose" | "list" | "settings" | "subject", action: () => void): void {
    const start = measureInput ? performance.now() : 0;
    action();
    if (measureInput) performance.measure("sprawling:shell_input_handler", { start, detail: name });
  }

  function collapse(restore: boolean): boolean {
    if (!narrow() || !expanded()) return false;
    setExpanded(false);
    document.removeEventListener("pointerdown", outside, true);
    if (restore) requestAnimationFrame(() => focusEdgeKey(ACTIONS_KEY, props.scope));
    return true;
  }

  function open(): void {
    setExpanded(true);
    document.addEventListener("pointerdown", outside, { capture: true, passive: true });
    requestAnimationFrame(() => focusEdgeKey(SETTINGS_KEY, props.scope));
  }

  function outside(event: PointerEvent): void {
    if (event.target instanceof Node && rail?.contains(event.target) === false) collapse(false);
  }

  onSettled(() => {
    const target = rail?.closest(".frame");
    const stop =
      target === null || target === undefined
        ? undefined
        : watchColumns(target, (cols) => {
            if (cols === "twelve") collapse(false);
            setNarrow(cols === "one");
          });
    const back = () => collapse(true);
    if (props.scope === undefined) backForEdge = back;
    return () => {
      stop?.();
      document.removeEventListener("pointerdown", outside, true);
      if (backForEdge === back) backForEdge = undefined;
    };
  });

  return (
    <div
      ref={(node) => {
        rail = node;
      }}
      class="refrain-controls"
      data-compact={compact() ? "" : undefined}
      data-expanded={narrow() && expanded() ? "" : undefined}
      onFocusOut={(event) => {
        if (
          event.relatedTarget instanceof Node &&
          !event.currentTarget.contains(event.relatedTarget)
        )
          collapse(false);
      }}
    >
      <button
        id={id(ACTIONS_KEY)}
        type="button"
        class="refrain-rail-key refrain-rail-launch glass"
        data-control=""
        aria-expanded={expanded() ? "true" : "false"}
        aria-controls={id(ACTION_GROUP)}
        aria-label={say(props.lang, "shell_edge_actions")}
        onClick={() => activate("disclose", open)}
      >
        <Glyph name="app-grid" size="key" />
      </button>
      <div
        id={id(ACTION_GROUP)}
        class="refrain-rail-actions"
        role={narrow() && expanded() ? "group" : undefined}
        aria-label={narrow() && expanded() ? say(props.lang, "shell_edge_actions") : undefined}
      >
        <button
          id={id(SETTINGS_KEY)}
          type="button"
          class="refrain-rail-key refrain-rail-action glass"
          data-control=""
          aria-label={say(props.lang, "nav_settings")}
          aria-haspopup="dialog"
          aria-expanded={props.settingsOpen ? "true" : "false"}
          hidden={compact()}
          onClick={(event) =>
            activate("settings", () => {
              props.onSettings(event.currentTarget);
              collapse(false);
            })
          }
        >
          <Glyph name="settings" size="key" />
          <span class="refrain-rail-caption" aria-hidden="true">
            {say(props.lang, "nav_settings")}
          </span>
        </button>
        <button
          id={id(NAVIGATION_KEY)}
          type="button"
          class="refrain-rail-key refrain-rail-action glass"
          data-control=""
          aria-label={listName()}
          aria-expanded={props.navigation ? "true" : "false"}
          aria-controls={id(NAVIGATION_ID)}
          hidden={compact()}
          onClick={(event) =>
            activate("list", () => {
              props.onNavigation(event.timeStamp);
              collapse(false);
            })
          }
        >
          <Glyph name={props.subject === "files" ? "folder-tree" : "session-list"} size="key" />
          <span class="refrain-rail-caption" aria-hidden="true">
            {listName()}
          </span>
        </button>
        <div
          class="refrain-rail-mailbox"
          onClick={(event) => {
            if (
              event.target instanceof Element &&
              event.target.closest(`#${id(MAILBOX_KEY)}, [data-refrain-mailbox-key]`) !== null
            )
              collapse(false);
          }}
        >
          {props.mailbox}
        </div>
        <button
          id={id(SUBJECT_KEY)}
          type="button"
          class="refrain-rail-key refrain-rail-action glass"
          data-control=""
          aria-label={say(props.lang, "shell_switch_subject")}
          aria-pressed={props.subject === "files" ? "true" : "false"}
          hidden={compact()}
          onClick={() =>
            activate("subject", () => {
              props.onSubject();
              collapse(true);
            })
          }
        >
          <Glyph name="switch-subject" size="key" />
          <span class="refrain-rail-caption" aria-hidden="true">
            {say(props.lang, "shell_switch_subject_short")}
          </span>
        </button>
      </div>
    </div>
  );
}

export function DensityKey(props: {
  readonly lang: Lang;
  readonly panorama: boolean;
  readonly onToggle: () => void;
  readonly onPeek: (peeking: boolean) => void;
}) {
  const [holding, setHolding] = createSignal(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let peeked = false;
  let pointer: number | null = null;

  function cancel(): void {
    if (pointer === null) return;
    pointer = null;
    clearTimeout(timer);
    timer = undefined;
    if (peeked) {
      setHolding(false);
      props.onPeek(false);
    }
  }

  function press(event: PointerEvent): void {
    if (event.button !== 0 || pointer !== null) return;
    pointer = event.pointerId;
    peeked = false;
    timer = setTimeout(() => {
      peeked = !props.panorama;
      if (peeked) {
        setHolding(true);
        props.onPeek(true);
      }
    }, HOLD_MS);
  }

  function density(event: MouseEvent): void {
    if (event.detail !== 0 && peeked) {
      peeked = false;
      return;
    }
    props.onToggle();
  }

  const release = (event: PointerEvent): void => {
    if (event.pointerId === pointer) cancel();
  };
  onSettled(() => {
    window.addEventListener("blur", cancel);
    window.addEventListener("pointerup", release);
    window.addEventListener("pointercancel", release);
    return () => {
      window.removeEventListener("blur", cancel);
      window.removeEventListener("pointerup", release);
      window.removeEventListener("pointercancel", release);
    };
  });
  onCleanup(cancel);

  return (
    <button
      type="button"
      class="refrain-nav-button"
      data-control="surface"
      aria-pressed={props.panorama ? "true" : "false"}
      data-control-holding={holding() ? "" : undefined}
      onPointerDown={press}
      onPointerLeave={cancel}
      onClick={density}
    >
      {say(props.lang, "tier_panorama")}
    </button>
  );
}
