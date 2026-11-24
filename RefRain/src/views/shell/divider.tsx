// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal } from "solid-js";

import { fill, say, type Lang } from "../../core/lang";
import {
  DEFAULT_SPLIT_LINE,
  DEFAULT_SPLIT_TOP,
  LAST_SPLIT_LINE,
  MAX_SPLIT_TOP,
  MIN_SPLIT_TOP,
  START_FILE_LINE,
} from "../../core/shell_visit";
import { NARROWEST } from "../../core/workbench";
import { dividerLookOf, lineAt } from "../world/divider";
import { scopedId } from "./chrome";

export function Divider(props: {
  readonly scope?: string | undefined;
  readonly position: number;
  readonly lang: Lang;
  readonly stacked: boolean;
  readonly onResize: (position: number) => void;
}) {
  let gutter: HTMLDivElement | undefined;
  const [dragging, setDragging] = createSignal(false);
  const move = (position: number): void => {
    const [min, max] = props.stacked
      ? [MIN_SPLIT_TOP, MAX_SPLIT_TOP]
      : [START_FILE_LINE + NARROWEST, LAST_SPLIT_LINE];
    props.onResize(Math.max(min, Math.min(max, position)));
  };
  const look = createMemo(() =>
    dividerLookOf(
      {
        label: fill(say(props.lang, "world_divider"), { pane: say(props.lang, "shell_file_mode") }),
        controls: scopedId("workspace-file-canvas", props.scope),
        orientation: props.stacked ? "horizontal" : "vertical",
        span: props.stacked ? props.position : props.position - START_FILE_LINE,
        minimum: props.stacked ? MIN_SPLIT_TOP : NARROWEST,
        widest: props.stacked ? MAX_SPLIT_TOP : LAST_SPLIT_LINE - START_FILE_LINE,
        increment: props.stacked ? 5 : 1,
        dragging: dragging(),
      },
      {
        set: (span) => move(props.stacked ? span : START_FILE_LINE + span),
        reset: () => move(props.stacked ? DEFAULT_SPLIT_TOP : DEFAULT_SPLIT_LINE),
        drag: setDragging,
        follow: (point) => {
          const grid = gutter?.parentElement;
          if (grid != null) move(props.stacked ? topShare(grid, point) : lineAt(grid, point));
        },
      },
    ),
  );
  return (
    <div
      ref={(node) => {
        gutter = node;
      }}
      class={[
        "z-(--layer-raised) flex",
        props.stacked
          ? "refrain-divider-stacked"
          : "row-[1/3] -ml-gutter w-gutter justify-self-start",
      ]}
      style={{ "grid-column": props.stacked ? undefined : `${props.position} / span 1` }}
    >
      <div
        role={look().wire.role}
        tabindex={look().wire.tabindex}
        aria-orientation={look().wire["aria-orientation"]}
        aria-label={look().wire["aria-label"]}
        aria-controls={look().wire["aria-controls"]}
        aria-valuenow={look().wire["aria-valuenow"]}
        aria-valuemin={look().wire["aria-valuemin"]}
        aria-valuemax={look().wire["aria-valuemax"]}
        onKeyDown={(event) => look().wire.onkeydown(event)}
        onDblClick={() => look().wire.ondblclick()}
        onPointerDown={(event) => look().wire.onpointerdown(event)}
        onPointerMove={(event) => look().wire.onpointermove(event)}
        onPointerUp={() => look().wire.onpointerup()}
        onPointerCancel={() => look().wire.onpointercancel()}
        class={[
          "relative flex flex-1 touch-none -outline-offset-2",
          props.stacked
            ? "refrain-divider-handle cursor-row-resize items-center"
            : "cursor-col-resize justify-center before:w-px",
          "before:transition-colors focus-visible:before:bg-accent",
          look().dragging
            ? "before:bg-accent before:ease-arrive"
            : "before:ease-leave hover:before:bg-edge-input hover:before:ease-arrive focus-visible:before:ease-arrive",
        ]}
      />
    </div>
  );
}

function topShare(grid: HTMLElement, clientY: number): number {
  const style = getComputedStyle(grid);
  const top = Number.parseFloat(style.paddingTop) || 0;
  const bottom = Number.parseFloat(style.paddingBottom) || 0;
  const usable = Math.max(1, grid.clientHeight - top - bottom);
  return Math.round((100 * (clientY - grid.getBoundingClientRect().top - top)) / usable);
}
