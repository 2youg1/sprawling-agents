// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "../../core/snippet";

export interface RowProps<Content = Snippet> {
  readonly primary: string;
  readonly secondary?: string;

  readonly status?: Content;

  readonly actions?: Content;

  readonly onOpen?: () => void;

  readonly chosen?: boolean;
}

export interface RowListProps {
  readonly label: string;

  readonly rows: Snippet;
}

export interface ItemWire {
  readonly "aria-current"?: "true";
}

export interface OpenWire {
  readonly type: "button";
  readonly onclick: () => void;
}

export interface RowLook<Content = Snippet> {
  readonly item: ItemWire;
  readonly chosen: boolean;
  readonly primary: string;
  readonly secondary: string | undefined;
  readonly status: Content | undefined;
  readonly actions: Content | undefined;
  readonly open: OpenWire | undefined;
}

export function lookOf<Content>(props: RowProps<Content>): RowLook<Content> {
  const chosen = props.chosen === true;
  const onOpen = props.onOpen;
  return {
    item: chosen ? { "aria-current": "true" } : {},
    chosen,
    primary: props.primary,
    secondary: props.secondary,
    status: props.status,
    actions: props.actions,
    open:
      onOpen === undefined
        ? undefined
        : {
            type: "button",
            onclick: () => {
              onOpen();
            },
          },
  };
}

export type Target = "row" | "text" | "select" | "editable";

export type Step = "next" | "previous" | "first" | "last";

export function taken(target: Target, key: string): Step | null {
  switch (target) {
    case "row":
      return stepOf(key);
    case "text":
    case "select":
    case "editable":
      return null;
  }
}

function stepOf(key: string): Step | null {
  switch (key) {
    case "ArrowDown":
      return "next";
    case "ArrowUp":
      return "previous";
    case "Home":
      return "first";
    case "End":
      return "last";
    default:
      return null;
  }
}

function courseOf(step: Step, from: number, last: number): readonly [number, number] {
  switch (step) {
    case "next":
      return [from + 1, 1];
    case "previous":
      return [from - 1, -1];
    case "first":
      return [0, 1];
    case "last":
      return [last, -1];
  }
}

export function landing<T>(
  reach: readonly (T | undefined)[],
  step: Step,
  from: number,
): T | undefined {
  const [start, by] = courseOf(step, from, reach.length - 1);
  for (let at = start; at >= 0 && at < reach.length; at += by) {
    const found = reach[at];
    if (found !== undefined) {
      return found;
    }
  }
  return undefined;
}

const REACHABLE = 'a[href], button:not([disabled]), [tabindex="0"]';

function targetOf(target: EventTarget | null): Target {
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
    return "text";
  }
  if (target instanceof HTMLSelectElement) {
    return "select";
  }
  return target instanceof HTMLElement && target.isContentEditable ? "editable" : "row";
}

export function walk(
  event: KeyboardEvent & { readonly currentTarget: EventTarget & HTMLUListElement },
): void {
  const step = taken(targetOf(event.target), event.key);
  if (step === null) {
    return;
  }
  const rows = [...event.currentTarget.children];
  const from = rows.findIndex((row) => event.target instanceof Node && row.contains(event.target));
  if (from < 0) {
    return;
  }
  const next = landing(
    rows.map((row) => {
      const reach = row.querySelector(REACHABLE);
      return reach instanceof HTMLElement ? reach : undefined;
    }),
    step,
    from,
  );
  if (next === undefined) {
    return;
  }
  event.preventDefault();
  next.focus();
}
