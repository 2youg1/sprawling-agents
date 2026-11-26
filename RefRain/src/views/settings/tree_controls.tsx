// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";

import Glyph from "../parts/glyph.solid";
import type { GlyphName } from "../parts/glyph";
import { KbdMarks } from "../parts/kbd.look";
import { chordMarks } from "../parts/kbd_marks";
import type {
  BranchLook,
  FoldLook,
  GroupLook,
  LeafLook,
  NestLook,
  PageLook,
  TreeLook,
} from "./tree";
import "./tree.css";

const ENTRY =
  "group/entry relative flex min-h-control-sm w-full items-center gap-snug rounded-control px-snug text-left text-label " +
  "text-text-quiet transition-[background-color,color,scale] ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive " +
  "active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100";
const MARK = "flex size-glyph-sm shrink-0 items-center justify-center text-text-faint";
const HERE = "chosen bg-raised text-text";
const CHILDREN = "drop flex flex-col gap-hair pb-base";
const NESTED = "drop flex flex-col gap-hair";
const DEPTH = "min-w-0 flex-1 truncate ps-pane";
const DEEPER = "min-w-0 flex-1 truncate ps-[calc(var(--spacing-pane)*2)]";

interface NavProps extends TreeLook {
  readonly hold?: (node: HTMLElement) => void;
}

function Lead(props: { readonly mark?: GlyphName | null }) {
  return (
    <span class={MARK} aria-hidden="true">
      <Show when={props.mark} keyed>
        {(mark) => <Glyph name={mark} size="sm" />}
      </Show>
    </span>
  );
}

function Disclosure(props: {
  readonly fold: FoldLook;
  readonly depth?: string;
  readonly mark?: GlyphName | null | undefined;
  readonly class?: string;
}) {
  return (
    <button
      type={props.fold.wire.type}
      data-control=""
      data-entry={props.fold.wire["data-entry"]}
      aria-expanded={props.fold.wire["aria-expanded"] ? "true" : "false"}
      aria-controls={props.fold.wire["aria-controls"]}
      onClick={() => props.fold.wire.onclick()}
      class={[ENTRY, props.class]}
    >
      <Lead mark={props.mark ?? null} />
      <span class={props.depth ?? "min-w-0 flex-1 truncate"}>{props.fold.label}</span>
      <span
        class={[MARK, "settings-tree-chevron chevron", props.fold.open && "shown"]}
        aria-hidden="true"
      >
        <Glyph name="chevron" size="sm" />
      </span>
    </button>
  );
}

function PageLink(props: { readonly row: PageLook; readonly deep: boolean }) {
  return (
    <a {...props.row.wire} class={[ENTRY, props.row.here && "text-text"]}>
      <Lead />
      <span class={props.deep ? DEEPER : DEPTH}>{props.row.label}</span>
      <Show when={props.row.reading !== null}>
        <span class="shrink-0 font-mono text-note text-text-faint">{props.row.reading}</span>
      </Show>
      <span class={MARK} aria-hidden="true">
        →
      </span>
    </a>
  );
}

function GroupButton(props: {
  readonly row: GroupLook;
  readonly deep: boolean;
  readonly mark?: GlyphName | null | undefined;
}) {
  return (
    <button
      type={props.row.wire.type}
      data-control=""
      data-entry={props.row.wire["data-entry"]}
      data-initial={props.row.wire["data-initial"]}
      aria-current={props.row.wire["aria-current"]}
      onClick={() => props.row.wire.onclick()}
      class={[ENTRY, props.row.here && HERE]}
    >
      <Lead mark={props.mark ?? null} />
      <span class={props.mark ? "min-w-0 flex-1 truncate" : props.deep ? DEEPER : DEPTH}>
        {props.row.label}
      </span>
      <span
        class={[
          MARK,
          "opacity-0 transition-opacity ease-leave group-hover/entry:opacity-100 group-focus-within/entry:opacity-100",
          props.row.here && "opacity-100",
        ]}
        aria-hidden="true"
      >
        <KbdMarks marks={chordMarks({ initial: props.row.initial })} />
      </span>
    </button>
  );
}

function Leaf(props: { readonly row: LeafLook | NestLook; readonly deep: boolean }) {
  const group = () => (props.row.kind === "group" ? props.row : null);
  const page = () => (props.row.kind === "page" ? props.row : null);
  const nest = () => (props.row.kind === "nest" ? props.row : null);
  return (
    <li>
      <Show when={group()}>{(row) => <GroupButton row={row()} deep={props.deep} />}</Show>
      <Show when={page()}>{(row) => <PageLink row={row()} deep={props.deep} />}</Show>
      <Show when={nest()}>
        {(row) => (
          <>
            <Disclosure fold={row().fold} depth={DEPTH} class="text-text-quiet" />
            <Show when={row().fold.open}>
              <ul id={row().fold.list} class={NESTED}>
                <For each={row().pages} keyed={(each) => each.key}>
                  {(each) => (
                    <li>
                      <PageLink row={each()} deep />
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          </>
        )}
      </Show>
    </li>
  );
}

function Branch(props: { readonly branch: BranchLook }) {
  return (
    <div class="flex flex-col gap-hair">
      <Disclosure
        fold={props.branch.fold}
        mark={props.branch.mark}
        class={
          props.branch.fold.open
            ? "text-note tracking-wide uppercase text-text-quiet"
            : "text-note tracking-wide uppercase text-text-faint"
        }
      />
      <Show when={props.branch.fold.open}>
        <ul id={props.branch.fold.list} class={CHILDREN}>
          <For each={props.branch.entries} keyed={(entry) => entry.key}>
            {(row) => <Leaf row={row()} deep={false} />}
          </For>
          <Show when={props.branch.more}>
            {(more) => (
              <li class="mt-hair">
                <Disclosure
                  fold={more().fold}
                  depth={DEPTH}
                  class="text-note text-text-faint hover:text-text-quiet"
                />
                <Show when={more().fold.open}>
                  <ul id={more().fold.list} class={NESTED}>
                    <For each={more().entries} keyed={(entry) => entry.key}>
                      {(row) => <Leaf row={row()} deep />}
                    </For>
                  </ul>
                </Show>
              </li>
            )}
          </Show>
        </ul>
      </Show>
    </div>
  );
}

export default function TreeControls(props: NavProps) {
  return (
    <nav
      ref={(node) => props.hold?.(node)}
      aria-label={props.nav["aria-label"]}
      onKeyDown={(event) => props.nav.onkeydown(event)}
      class="flex flex-col gap-tight"
    >
      <For each={props.branches} keyed={(branch) => branch.key}>
        {(branch) => (
          <Show when={branch().direct} fallback={<Branch branch={branch()} />}>
            {(row) => <GroupButton row={row()} mark={branch().mark} deep={false} />}
          </Show>
        )}
      </For>
    </nav>
  );
}
