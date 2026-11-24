// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { keymap, marks } from "../../core/keys";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { lineIn } from "../../core/stretches";
import { ui } from "../../ui";
import type { Address, Seq } from "../../wire";
import { openPalette, type Arrival } from "../palette_door";
import {
  addressLineOf,
  boxWords,
  type AddressLineLook,
  type At,
  type BoxWords,
  type Lesson,
} from "./address_line";

type Seat = { readonly at: "head" } | ({ readonly at: "box" } & BoxWords);

function Hint(props: { readonly lesson: Lesson | undefined }) {
  return (
    <Show when={props.lesson}>
      {(lesson) => (
        <span class="min-w-0 shrink-10000 truncate text-text-faint" aria-hidden="true">
          {"\u2003"}
          {lesson().words}
          <Show when={lesson().chord !== ""}>
            <span class="pointer-coarse:hidden">
              {"\u2002"}
              {lesson().chord}
            </span>
          </Show>
        </span>
      )}
    </Show>
  );
}

function AddressButton(props: {
  readonly look: AddressLineLook;
  readonly size?: string;
  readonly lesson?: Lesson | undefined;
}) {
  return (
    <button
      type={props.look.wire.type}
      data-control=""
      aria-haspopup={props.look.wire["aria-haspopup"]}
      aria-label={props.look.wire["aria-label"]}
      onClick={() => props.look.wire.onclick()}
      onPointerDown={(event) => props.look.wire.onpointerdown(event)}
      class={[
        "group relative flex max-w-full min-w-0 items-baseline rounded-control text-text-faint",
        props.size,
        "transition-[color,transform] duration-short ease-leave hover:text-text-quiet hover:ease-arrive focus-visible:text-text-quiet",
        "active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100",
        "pointer-coarse:before:absolute pointer-coarse:before:top-1/2 pointer-coarse:before:left-1/2 pointer-coarse:before:size-touch pointer-coarse:before:-translate-1/2 pointer-coarse:before:content-['']",
      ]}
    >
      <Show when={props.look.building}>
        {(building) => <span class="min-w-0 shrink-1000 truncate">{building()}</span>}
      </Show>
      <span class="min-w-0 truncate text-text-quiet group-hover:text-text">{props.look.room}</span>
      <Show when={props.look.session}>
        {(session) => (
          <>
            <span class="shrink-0 px-tight" aria-hidden="true">
              ·
            </span>
            <span class="figure shrink-0">{session()}</span>
          </>
        )}
      </Show>
      <Hint lesson={props.lesson} />
    </button>
  );
}

export function AddressLineLookView(props: {
  readonly look: AddressLineLook;
  readonly seat: Seat;
  readonly lesson?: Lesson | undefined;
}) {
  return (
    <Show
      when={props.seat.at === "box" ? props.seat : undefined}
      fallback={
        <header aria-label={props.look.head} class="fade flex h-control shrink-0 items-center">
          <AddressButton look={props.look} size="text-note" lesson={props.lesson} />
        </header>
      }
    >
      {(box) => (
        <span class="flex max-w-full min-w-0 items-baseline">
          <span class="shrink-0 whitespace-pre" aria-hidden="true">
            {box().before}
          </span>
          <span class="pointer-events-auto flex min-w-0">
            <AddressButton look={props.look} />
          </span>
          <span class="shrink-0 whitespace-pre" aria-hidden="true">
            {box().after}
          </span>
          <Hint lesson={props.lesson} />
        </span>
      )}
    </Show>
  );
}

export default function AddressLine(props: {
  readonly address: Address;
  readonly session?: Seq | undefined;
  readonly at?: At | undefined;
  readonly onOpen?: ((arrival: Arrival) => void) | undefined;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const found = storeValue(u.prefs.paletteFound);
  const lines = storeValue(() => u.conn.asking.ask({ sessions: { room: props.address } }));
  const keys = keymap();
  const session = createMemo(() => {
    const answer = lines();
    return (
      props.session ??
      lineIn(answer !== undefined && "sessions" in answer ? answer.sessions : undefined, undefined)
        ?.began ??
      null
    );
  });
  const look = createMemo(() =>
    addressLineOf(lang(), props.address, session(), props.onOpen ?? openPalette),
  );
  const seat = createMemo<Seat>(() =>
    props.at === "box" ? { at: "box", ...boxWords(lang()) } : { at: "head" },
  );
  const lesson = createMemo(() =>
    found()
      ? undefined
      : {
          words: say(lang(), "talk_address_hint"),
          chord: marks(keys.chord("palette"), keys.platform).join(" "),
        },
  );
  return <AddressLineLookView look={look()} seat={seat()} lesson={lesson()} />;
}
