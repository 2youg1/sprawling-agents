// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import { face, marks, platformOf } from "../../core/keys";
import type { ContextTool, ContextToolsController } from "./context_tools";
import Glyph from "./glyph.solid";

export function wanted(value: ContextTool["disabled"]): boolean {
  return typeof value === "function" ? value() : (value ?? false);
}

export function opens(tool: ContextTool): boolean {
  return tool.panel !== undefined || (tool.children?.length ?? 0) > 0;
}

export function ContextRow(props: {
  readonly tool: ContextTool;
  readonly disabled: boolean;
  readonly controller: ContextToolsController;
}) {
  const held = () => props.tool.hold !== undefined;
  const platform = platformOf(navigator.userAgent);
  const shortcut = () => props.tool.shortcut;
  const spokenShortcut = () => {
    const chord = shortcut();
    if (chord === undefined) return undefined;
    return [
      ...(chord.accel ? [platform === "mac" ? "Meta" : "Control"] : []),
      ...(chord.shift ? ["Shift"] : []),
      face(chord.key),
    ].join("+");
  };
  let pointer: number | undefined;
  const down = (event: PointerEvent & { currentTarget: HTMLButtonElement }): void => {
    if (!held() || props.disabled || event.button !== 0 || !event.isPrimary) return;
    pointer = event.pointerId;
    event.currentTarget.setPointerCapture(event.pointerId);
    props.controller.hold(props.tool);
  };
  const release = (): void => {
    pointer = undefined;
    if (held()) props.controller.hold(undefined);
  };
  const move = (event: PointerEvent & { currentTarget: HTMLButtonElement }): void => {
    if (pointer !== event.pointerId) return;
    const { left, right, top, bottom } = event.currentTarget.getBoundingClientRect();
    if (
      event.clientX < left ||
      event.clientX > right ||
      event.clientY < top ||
      event.clientY > bottom
    )
      release();
  };
  const holdKey = (event: KeyboardEvent, enabled: boolean): void => {
    if (!held() || props.disabled || (event.key !== " " && event.key !== "Enter")) return;
    event.preventDefault();
    event.stopPropagation();
    if (enabled) props.controller.hold(props.tool);
    else release();
  };
  return (
    <button
      type="button"
      tabindex={-1}
      data-context-item={props.tool.id}
      data-control="surface"
      data-control-holding={props.controller.holding() === props.tool.id ? "" : undefined}
      role={props.tool.checked === undefined ? "menuitem" : "menuitemcheckbox"}
      aria-checked={
        props.tool.checked === undefined ? undefined : wanted(props.tool.checked) ? "true" : "false"
      }
      aria-haspopup={
        props.tool.panel === undefined ? (opens(props.tool) ? "menu" : undefined) : "dialog"
      }
      aria-disabled={props.disabled ? "true" : undefined}
      aria-keyshortcuts={spokenShortcut()}
      onClick={() => {
        props.controller.enter(props.tool);
      }}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={release}
      onPointerCancel={release}
      onLostPointerCapture={release}
      onBlur={release}
      onKeyDown={(event) => {
        holdKey(event, true);
      }}
      onKeyUp={(event) => {
        holdKey(event, false);
      }}
    >
      <span class="flex min-w-0 items-center gap-snug">
        <Show when={props.tool.glyph}>{(name) => <Glyph name={name()} size="sm" />}</Show>
        <span>{props.tool.label}</span>
      </span>
      <Show when={shortcut()}>
        {(chord) => <kbd aria-hidden="true">{marks(chord(), platform).join("+")}</kbd>}
      </Show>
      <Show when={wanted(props.tool.checked)}>
        <span aria-hidden="true">✓</span>
      </Show>
      <Show when={opens(props.tool)}>
        <span aria-hidden="true">›</span>
      </Show>
    </button>
  );
}
