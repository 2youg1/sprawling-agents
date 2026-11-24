// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option } from "effect";
import { createSignal, For, Show } from "solid-js";
import { fill, say } from "../../core/lang";
import { go, roomIn, roomOf } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { Address, type BuildingAnswer } from "../../wire";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
import RoomRow from "./room_row.look.solid";

interface RoomsProps {
  readonly answer: BuildingAnswer;
  readonly living: (room: Address) => number;
  readonly onPick: (room: Address) => void;
}

export default function Rooms(props: RoomsProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const [roomName, setRoomName] = createSignal("");
  const named = () => roomIn(props.answer.addr, roomName());
  let naming: HTMLFormElement | undefined;

  return (
    <div>
      <h2 class="mb-snug text-note text-text-faint">{say(lang(), "bld_rooms")}</h2>
      <Show
        when={props.answer.rooms.length > 0}
        fallback={
          <Button
            label={say(lang(), "bld_no_rooms")}
            tone="primary"
            onPress={() => naming?.querySelector("input")?.focus()}
          />
        }
      >
        <ul class="text-note">
          <For each={props.answer.rooms} keyed>
            {(name) => {
              const room = () => Address.make(`${props.answer.addr}/${name}`);
              return (
                <li>
                  <RoomRow
                    name={roomOf(room())}
                    live={
                      props.living(room()) > 0
                        ? fill(say(lang(), "city_active"), { n: String(props.living(room())) })
                        : undefined
                    }
                    wire={{ type: "button", onclick: () => props.onPick(room()) }}
                  />
                </li>
              );
            }}
          </For>
        </ul>
      </Show>
      <form
        class="mt-base flex flex-wrap items-end gap-snug"
        ref={(node) => {
          naming = node;
        }}
        onSubmit={(event) => {
          event.preventDefault();
          const room = named();
          if (Option.isNone(room)) return;
          go(u.bar, { kind: "talk", address: room.value });
          setRoomName("");
        }}
      >
        <div class="min-w-[min(100%,26ch)] flex-1">
          <Field
            label={say(lang(), "bld_room_name")}
            help={say(lang(), "bld_room_name_help")}
            placeholder={say(lang(), "bld_room_name_example")}
            value={roomName()}
            onInput={setRoomName}
          />
        </div>
        <Button
          label={say(lang(), "bld_room_talk")}
          type="submit"
          tone={Option.isSome(named()) ? "primary" : "secondary"}
        />
      </form>
    </div>
  );
}
