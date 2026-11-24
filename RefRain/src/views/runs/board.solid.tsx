// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option } from "effect";
import { createEffect, createMemo, createSignal, createUniqueId, onCleanup } from "solid-js";
import { lineWalker } from "../../core/lines";
import { pressedOf } from "../../core/press";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { lookOf, type TreeHands } from "./board";
import Look from "./board.look.solid";
import { phaseOf, rowsOf, viewOf, windowOf, type BoardRun, type Row } from "./lineage";

export interface RunsBoardProps {
  readonly runs: readonly BoardRun[];
  readonly now: number;
  readonly level?: 1 | 2;
}

const MARGIN = 12;

interface Geometry {
  readonly row: number;
  readonly height: number;
}

export default function RunsBoard(props: RunsBoardProps) {
  const u = ui();
  const id = createUniqueId();
  const lang = storeValue(u.lang);
  const [folded, setFolded] = createSignal<ReadonlySet<string>>(new Set());
  const [scrolled, setScrolled] = createSignal(0);
  const [geometry, setGeometry] = createSignal<Geometry>({ row: 0, height: 0 });
  const [list, setList] = createSignal<HTMLUListElement>();
  const [chosen, setChosen] = createSignal<string | null>(null);
  const rows = createMemo(() => rowsOf(props.runs, folded()));
  const firstWaiting = createMemo(
    () => props.runs.find((run) => phaseOf(run.doing) === "person")?.run,
  );
  const visible = createMemo(() => {
    const { row, height } = geometry();
    return windowOf(rows().length, scrolled(), height, row, MARGIN);
  });
  const cursor = createMemo(() =>
    Math.max(
      0,
      rows().findIndex((row) => row.key === (chosen() ?? firstWaiting())),
    ),
  );
  const walk = lineWalker();
  let seek = 0;
  onCleanup(() => cancelAnimationFrame(seek));

  createEffect(
    () => [list(), rows().length] as const,
    ([node, total]) => {
      if (node === undefined || total === 0) return;
      let observed: HTMLLIElement | null = null;
      let frame = 0;
      const measure = (): void => {
        frame = 0;
        const first = node.querySelector("li");
        if (first !== observed) {
          if (observed !== null) observer.unobserve(observed);
          if (first !== null) observer.observe(first);
          observed = first;
        }
        const row = first?.getBoundingClientRect().height ?? 0;
        const height = node.clientHeight;
        if (row > 0 && height > 0) {
          setGeometry((previous) =>
            previous.row === row && previous.height === height ? previous : { row, height },
          );
        }
      };
      const observer = new ResizeObserver(() => {
        if (frame === 0) frame = requestAnimationFrame(measure);
      });
      measure();
      observer.observe(node);
      return () => {
        cancelAnimationFrame(frame);
        observer.disconnect();
      };
    },
  );

  function moveTo(index: number): void {
    const at = Math.min(Math.max(index, 0), rows().length - 1);
    const row = rows()[at];
    if (row === undefined) return;
    setChosen(row.key);
    const node = list();
    const { row: stride, height } = geometry();
    if (node === undefined || stride === 0) return;
    const top = at * stride;
    if (top < node.scrollTop) node.scrollTop = top;
    else if (top + stride > node.scrollTop + height) node.scrollTop = top + stride - height;
  }

  function foldAt(key: string, shut: boolean): void {
    setFolded((held) => {
      if (held.has(key) === shut) return held;
      const next = new Set(held);
      if (shut) next.add(key);
      else next.delete(key);
      return next;
    });
  }

  function folding(event: KeyboardEvent, row: Row | undefined): void {
    switch (event.key) {
      case "h":
      case "ArrowLeft":
        if (row !== undefined && row.kind !== "run") foldAt(row.key, true);
        break;
      case "l":
      case "ArrowRight":
        if (row !== undefined && row.kind !== "run") foldAt(row.key, false);
        break;
      default:
        return;
    }
    event.preventDefault();
  }

  function pressed(event: KeyboardEvent): void {
    const row = rows()[cursor()];
    switch (walk(pressedOf(event), event.timeStamp)) {
      case "line.next":
        moveTo(cursor() + 1);
        break;
      case "line.previous":
        moveTo(cursor() - 1);
        break;
      case "line.first":
        moveTo(0);
        break;
      case "line.last":
        moveTo(rows().length - 1);
        break;
      case "line.open":
        if (row !== undefined) open(row);
        break;
      case "line.close":
      case null:
        folding(event, row);
        return;
    }
    event.preventDefault();
  }

  function open(row: Row): void {
    setChosen(row.key);
    Option.map(viewOf(row), u.go);
  }

  function pick(run: BoardRun): void {
    const room = run.addr ?? "";
    const building = room.split("/")[0] ?? "";
    foldAt(building, false);
    foldAt(room, false);
    setChosen(run.run);
    cancelAnimationFrame(seek);
    seek = requestAnimationFrame(() => {
      const target = rows().findIndex((row) => row.key === run.run);
      if (target !== -1) moveTo(target);
    });
    list()?.focus({ preventScroll: true });
  }

  const tree: TreeHands = {
    onkeydown: pressed,
    onscroll: (event) => {
      setScrolled(event.currentTarget.scrollTop);
    },
  };
  const look = createMemo(() =>
    lookOf(
      {
        id,
        runs: props.runs,
        now: props.now,
        lang: lang(),
        level: props.level ?? 1,
        rows: rows(),
        folded: folded(),
        cursor: cursor(),
        shown: visible(),
        rowPx: geometry().row,
      },
      { tree, open, pick },
    ),
  );
  return <Look look={look()} hold={setList} />;
}
