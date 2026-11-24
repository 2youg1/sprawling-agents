// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, Show, untrack } from "solid-js";

import { newestWorking } from "../../core/belief/live";
import { heldIn } from "../../core/belief/rooms";
import { cancel } from "../../core/commands";
import { say } from "../../core/lang";
import { buildingOf } from "../../core/route";
import { collapsedOf, sessionTree } from "../../core/session_tree";
import { pinningOf, stretchesOf, tailOf, type Stretch } from "../../core/stretches";
import { inUse, namedIn, tagsOf } from "../../core/tags";
import { ui } from "../../ui";
import type { Address, Seq, SessionsAnswer, Tag } from "../../wire";
import Button from "../parts/button.solid";
import Glyph from "../parts/glyph.solid";
import { branchFrom } from "../talk/branching";
import { ticker } from "../talk/timing";
import ContextBar from "../world/context_bar_view";
import SessionMenu from "../world/session_menu_view";
import { dotOf, filterOf, rowOf, type RowLook } from "../world/sessions";
import { storeValue } from "../../core/solid_store";

export function SessionList(props: { readonly here: Address; readonly session?: Seq | undefined }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const knownTags = storeValue(u.tags.held);
  const branching = storeValue(u.branches.held);
  const clock = storeValue(ticker(u.now));
  const [answers, setAnswers] = createSignal<Readonly<Record<string, SessionsAnswer>>>({});
  const [filter, setFilter] = createSignal<Tag | null>(null);
  const [overrides, setOverrides] = createSignal<Readonly<Record<string, boolean>>>({});

  const rooms = createMemo(() =>
    [...belief().rooms.keys()].flatMap((room) => {
      const address = heldIn(belief(), room).at(-1)?.addr;
      return address !== undefined &&
        address !== null &&
        buildingOf(address) === buildingOf(props.here)
        ? [address]
        : [];
    }),
  );
  const roomsKey = createMemo(() => rooms().join("\n"));
  const listed = createMemo(() =>
    stretchesOf(Object.values(answers()), (room) => heldIn(belief(), room)).filter(
      (stretch) => buildingOf(stretch.room) === buildingOf(props.here),
    ),
  );

  function name(stretch: Stretch) {
    return namedIn(belief().city, stretch.room, stretch.line.began, stretch.line.workspace ?? null);
  }

  function tags(stretch: Stretch): readonly Tag[] {
    const named = name(stretch);
    return named === null ? [] : tagsOf(knownTags(), named);
  }

  const offered = createMemo(() => (belief().city === null ? [] : inUse(listed().map(tags))));
  const by = createMemo(() => {
    const candidate = filter();
    return candidate !== null && offered().includes(candidate) ? candidate : null;
  });
  const filtered = createMemo(() => {
    const active = by();
    return active === null
      ? listed()
      : listed().filter((stretch) => tags(stretch).includes(active));
  });
  const collapsed = createMemo(() => collapsedOf(filtered(), overrides()));
  const running = createMemo(() => filtered().some((stretch) => dotOf(stretch) === "run"));
  const switches = createMemo(() =>
    filterOf(
      offered(),
      by(),
      {
        label: say(lang(), "world_tags"),
        all: say(lang(), "world_tags_all"),
      },
      (tag) => {
        setFilter(tag);
      },
    ),
  );
  const rows = createMemo(() =>
    sessionTree(filtered(), collapsed()).map((node) => {
      const stretch = node.stretch;
      const lessons = tags(stretch);
      const origin = tailOf(stretch);
      const going = newestWorking(belief(), stretch.room);
      return rowOf(stretch, {
        lang: lang(),
        now: running() ? clock() : u.now(),
        chosen:
          stretch.room === props.here &&
          (props.session === undefined ? stretch.current : stretch.line.began === props.session),
        tags: lessons,
        pinning: pinningOf(stretch, lessons),
        named: name(stretch),
        depth: node.depth,
        children: node.children,
        collapsed: collapsed().has(node.key),
        onToggle: () => {
          setOverrides((held) => ({ ...held, [node.key]: !collapsed().has(node.key) }));
        },
        branch: origin === null ? { kind: "none" } : branchFrom(branching(), stretch.room, origin),
        stop:
          going === undefined
            ? null
            : () => {
                u.send(cancel(going.run));
              },
      });
    }),
  );

  createEffect(roomsKey, (key) => {
    const active = key === "" ? [] : untrack(rooms);
    const stops = active.map((room) =>
      u.conn.asking.ask({ sessions: { room } }).subscribe((answer) => {
        if (answer !== undefined && "sessions" in answer)
          setAnswers((values) => ({ ...values, [room]: answer.sessions }));
      }),
    );
    return () => {
      for (const stop of stops) stop();
    };
  });

  return (
    <section
      class="flex min-h-0 flex-1 flex-col overflow-hidden px-snug"
      aria-label={say(lang(), "world_sessions")}
    >
      <Show when={switches()}>
        {(group) => (
          <div
            role="group"
            aria-label={group().wire["aria-label"]}
            class="mb-snug flex flex-wrap gap-tight"
          >
            <For each={group().toggles} keyed={(toggle) => toggle.key}>
              {(toggle) => (
                <button
                  data-control=""
                  type="button"
                  aria-pressed={toggle().wire["aria-pressed"] ? "true" : "false"}
                  onClick={() => toggle().wire.onclick()}
                  class="h-control-sm rounded-pill px-snug text-note text-text-quiet hover:wash hover:text-text aria-pressed:wash-strong aria-pressed:text-text"
                >
                  {toggle().word}
                </button>
              )}
            </For>
          </div>
        )}
      </Show>
      <div class="-mx-snug min-h-0 flex-1 overflow-y-auto px-snug">
        <ul>
          <For each={rows()} keyed={(row) => row.key}>
            {(row) => <SessionRow row={row()} />}
          </For>
        </ul>
      </div>
    </section>
  );
}

function SessionRow(props: { readonly row: RowLook }) {
  const indent = () => `calc(${props.row.depth} * var(--spacing-base))`;
  return (
    <li
      class={[
        "refrain-session-row group relative -mx-snug flex items-start rounded-card",
        props.row.chosen ? "here wash-strong" : "hover:wash",
      ]}
    >
      <Show
        when={props.row.disclosure}
        fallback={
          <span class="w-control-sm shrink-0" style={{ "margin-inline-start": indent() }} />
        }
      >
        {(disclosure) => (
          <button
            data-control=""
            type="button"
            aria-label={disclosure()["aria-label"]}
            aria-expanded={disclosure()["aria-expanded"] ? "true" : "false"}
            onClick={() => disclosure().onclick()}
            class="mt-snug grid size-control-sm shrink-0 place-items-center rounded-control text-text-faint hover:wash hover:text-text"
            style={{ "margin-inline-start": indent() }}
          >
            <span class={disclosure()["aria-expanded"] ? "rotate-90" : ""}>
              <Glyph name="chevron" size="sm" />
            </span>
          </button>
        )}
      </Show>
      <a
        href={props.row.link.href}
        aria-current={props.row.link["aria-current"]}
        aria-busy={props.row.link["aria-busy"] ? "true" : "false"}
        onClick={(event) => props.row.link.onclick(event)}
        class="refrain-session-cells grid min-w-0 flex-1 gap-x-base rounded-card px-snug py-snug"
      >
        <span
          class="refrain-session-dot mt-snug size-dot rounded-pill"
          data-dot={props.row.dot}
          aria-hidden="true"
        />
        <span class="flex min-w-0 items-center gap-tight">
          <span
            class={["truncate font-label", props.row.current ? "text-text" : "text-text-quiet"]}
            title={props.row.room}
          >
            {props.row.title}
          </span>
          <Show when={props.row.pin}>
            {(pin) => (
              <span
                role="img"
                aria-label={pin().wire["aria-label"]}
                title={pin().hint}
                class="flex shrink-0 text-text-faint"
              >
                <Glyph name="pin" size="sm" />
              </span>
            )}
          </Show>
        </span>
        <span class="figure text-note text-text-faint">{props.row.when ?? ""}</span>
        <Show when={props.row.phase}>
          <span class="col-start-2 col-end-4 truncate text-note text-text-quiet">
            {props.row.phase}
          </span>
        </Show>
        <Show when={props.row.delegated}>
          <span class="col-start-2 col-end-4 truncate text-note text-text-faint">
            {props.row.delegated}
          </span>
        </Show>
        <Show when={props.row.about}>
          <span class="col-start-2 col-end-4 line-clamp-2 text-note text-text-quiet">
            {props.row.about}
          </span>
        </Show>
        <Show when={props.row.ranOn}>
          <span class="col-start-2 col-end-4 truncate text-note text-text-faint">
            {props.row.ranOn}
          </span>
        </Show>
        <Show when={props.row.tags.length > 0}>
          <span class="col-start-2 col-end-4 mt-tight flex flex-wrap gap-tight">
            <For each={props.row.tags}>
              {(tag) => (
                <span class="rounded-pill border border-edge-panel px-tight text-note text-text-faint">
                  {tag}
                </span>
              )}
            </For>
          </span>
        </Show>
        <Show when={props.row.context}>
          {(context) => <ContextBar room={context().room} run={context().run} />}
        </Show>
      </a>
      <div
        class={[
          "refrain-session-menu shrink-0 pt-tight pr-tight",
          props.row.chosen
            ? ""
            : "opacity-0 transition-opacity ease-leave group-hover:opacity-100 group-hover:ease-arrive group-focus-within:opacity-100 group-focus-within:ease-arrive",
        ]}
      >
        <SessionMenu {...props.row.menu} />
      </div>
      <Show when={props.row.note}>
        {(note) => (
          <div
            class="flex w-full flex-wrap items-center gap-snug pb-snug pl-base text-note text-text-quiet"
            aria-live="polite"
          >
            <span>{note().said}</span>
            <Show when={note().stop}>
              {(stop) => (
                <Button label={stop().label} name={stop().name} onPress={stop().onPress} />
              )}
            </Show>
          </div>
        )}
      </Show>
    </li>
  );
}
