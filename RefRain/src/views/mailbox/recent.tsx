// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, createUniqueId, For, Show } from "solid-js";
import { newestWorking } from "../../core/belief/live";
import { cancel } from "../../core/commands";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { tailOf } from "../../core/stretches";
import { ago } from "../../core/time";
import { ui } from "../../ui";
import type { Address, Origin, SessionLine } from "../../wire";
import Button from "../parts/button.solid";
import ControlTip from "../parts/control_tip";
import Glyph from "../parts/glyph.solid";
import { OVERSCAN, windowOf } from "../run/lanes";
import { branchFrom, type BranchHere } from "../talk/branching";
import { askStretches } from "../world/stretches";
import { rowOf, type RecentRowLook } from "./recent_row";
import Section from "./section";

interface Session {
  readonly room: Address;
  readonly line: SessionLine;
  readonly origin: Origin | null;
  readonly text: string;
}

export function RecentRow(props: RecentRowLook) {
  const id = createUniqueId();
  return (
    <li class="-mx-snug flex h-control items-center gap-snug rounded-card px-snug hover:wash">
      <a
        class="grid min-w-0 flex-1 grid-cols-[minmax(10ch,1fr)_minmax(0,auto)_var(--spacing-figure)] items-center gap-x-base rounded-control whitespace-nowrap focus-visible:wash"
        href={props.link.href ?? undefined}
        data-entry=""
        onClick={(event) => {
          props.link.onclick?.call(event.currentTarget, event);
        }}
      >
        <span class="min-w-0 truncate">{props.room}</span>
        <span class="truncate text-note text-text-faint" aria-live="polite">
          {props.began}
        </span>
        <span class="figure text-right text-note text-text-faint">{props.ago}</span>
      </a>
      <ControlTip id={id} text={props.fork.why ?? props.fork.name}>
        <button
          class="grid size-control-sm shrink-0 place-items-center rounded-control text-text-quiet transition-colors duration-short ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive aria-disabled:text-text-disabled"
          type="button"
          data-control=""
          aria-label={props.fork.name}
          aria-describedby={id}
          aria-disabled={props.fork.wire["aria-disabled"] ? "true" : "false"}
          onClick={(event) => {
            props.fork.wire.onclick?.call(event.currentTarget, event);
          }}
        >
          <Glyph name="branch" size="sm" />
        </button>
      </ControlTip>
      <Show when={props.stop}>
        {(action) => (
          <Button label={action().label} name={action().name} onPress={action().onPress} />
        )}
      </Show>
      <kbd class="entry-n" aria-hidden="true" />
    </li>
  );
}

export default function Recent(props: {
  readonly scroller: () => HTMLElement | undefined;
  readonly onLeave: () => void;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const branching = storeValue(u.branches.held);
  const stretches = storeValue(askStretches(u));
  const [search, setSearch] = createSignal("");
  const [list, setList] = createSignal<HTMLUListElement>();
  const [shown, setShown] = createSignal({ first: 0, end: OVERSCAN });
  const [rowPx, setRowPx] = createSignal(0);
  const reach = `${createUniqueId()}-reach`;
  const rows = createMemo(() => {
    const needle = search().trim().toLowerCase();
    return stretches()
      .all.map((stretch): Session => ({
        room: stretch.room,
        line: stretch.line,
        origin: tailOf(stretch),
        text: [stretch.room, ...stretch.runs.map((each) => each.task ?? "")]
          .join("\n")
          .toLowerCase(),
      }))
      .filter((row) => row.text.includes(needle));
  });
  function fork(row: Session): void {
    if (row.origin !== null)
      u.branches.open(row.room, { kind: "branch", origin: row.origin }, props.onLeave);
  }
  function branchOf(row: Session): BranchHere {
    return row.origin === null ? { kind: "none" } : branchFrom(branching(), row.room, row.origin);
  }
  function stopIn(room: Address): (() => void) | null {
    const going = newestWorking(belief(), room);
    return going === undefined
      ? null
      : () => {
          u.send(cancel(going.run));
        };
  }
  createEffect(
    () => ({ held: list(), column: props.scroller(), total: rows().length }),
    ({ held, column, total }) => {
      if (held === undefined || column === undefined) return;
      const measure = (): void => {
        const height = Number.parseFloat(
          getComputedStyle(held).getPropertyValue("--spacing-control"),
        );
        const top = held.getBoundingClientRect().top - column.getBoundingClientRect().top;
        setRowPx(height);
        setShown(windowOf(total, height, Math.max(0, -top), column.clientHeight));
      };
      measure();
      const watching = new ResizeObserver(measure);
      watching.observe(column);
      column.addEventListener("scroll", measure, { passive: true });
      return () => {
        watching.disconnect();
        column.removeEventListener("scroll", measure);
      };
    },
  );
  return (
    <Section
      title={say(lang(), "mailbox_recent")}
      empty={say(lang(), search().trim() === "" ? "mailbox_recent_none" : "mailbox_search_none")}
      count={rows().length}
    >
      <Show when={stretches().all.length > 0}>
        <input
          type="search"
          class="mt-snug w-full rounded-control bg-page px-snug py-tight text-label placeholder:text-text-faint"
          aria-label={say(lang(), "mailbox_search")}
          aria-describedby={reach}
          placeholder={say(lang(), "mailbox_search")}
          value={search()}
          onInput={(event) => {
            setSearch(event.currentTarget.value);
          }}
        />
        <p id={reach} class="py-tight text-note text-text-faint">
          {say(lang(), "mailbox_search_reach")}
        </p>
      </Show>
      <ul
        ref={(node) => {
          setList(node);
        }}
        style={{
          height: `${String(rows().length * rowPx())}px`,
          "padding-top": `${String(shown().first * rowPx())}px`,
        }}
      >
        <For
          each={rows().slice(shown().first, shown().end)}
          keyed={(row) => `${row.room}@${String(row.line.began)}`}
        >
          {(row) => {
            const branch = createMemo(() => branchOf(row()));
            return (
              <RecentRow
                {...rowOf(
                  {
                    room: row().room,
                    line: row().line,
                    forkable: row().origin !== null,
                    branch: branch(),
                  },
                  lang(),
                  ago(lang(), row().line.at, u.now()),
                  {
                    href: toFragment({
                      kind: "talk",
                      address: row().room,
                      session: row().line.began,
                    }),
                    follow: props.onLeave,
                    fork: () => fork(row()),
                    stop: branch().kind === "refused" ? stopIn(row().room) : null,
                  },
                )}
              />
            );
          }}
        </For>
      </ul>
      <Show when={stretches().earlier > 0}>
        <p class="py-snug text-note text-text-faint">
          {fill(say(lang(), "mailbox_earlier"), { n: String(stretches().earlier) })}
        </p>
      </Show>
    </Section>
  );
}
