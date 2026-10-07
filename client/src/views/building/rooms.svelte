<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The rooms of one building, each a way into its directory, and the
  // field that opens a new one. A room is opened by talking in it: the
  // talk page's first dispatch to `<building>/<name>` opens that room,
  // so no model has to name it.
  import { Option } from "effect";
  import { fill, say } from "../../core/lang";
  import { go, roomIn, roomOf } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, BuildingAnswer } from "../../wire";
  import { Address as AddressSchema } from "../../wire";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import RoomRow from "./room_row.look.svelte";

  interface RoomsProps {
    readonly answer: BuildingAnswer;
    // How many runs are working at or below a room, which the list
    // lights its dots for.
    readonly living: (room: Address) => number;
    readonly onPick: (room: Address) => void;
  }

  const { answer, living, onPick }: RoomsProps = $props();

  const u = ui();
  const lang = u.lang;
  let roomName = $state("");
  const named = $derived(roomIn(answer.addr, roomName));

  function talkIn(): void {
    if (Option.isNone(named)) return;
    go(u.bar, { kind: "talk", address: named.value });
    roomName = "";
  }
</script>

<div>
  <h2 class="mb-snug text-note text-text-faint">{say($lang, "bld_rooms")}</h2>
  {#if answer.rooms.length > 0}
    <ul class="text-note">
      {#each answer.rooms as name (name)}
        {@const room = AddressSchema.make(`${answer.addr}/${name}`)}
        <li>
          <RoomRow
            name={roomOf(room)}
            live={living(room) > 0 ? fill(say($lang, "city_active"), { n: String(living(room)) }) : undefined}
            wire={{
              type: "button",
              onclick: () => {
                onPick(room);
              },
            }}
          />
        </li>
      {/each}
    </ul>
  {:else}
    <p class="text-note text-text-faint">{say($lang, "bld_no_rooms")}</p>
  {/if}
  <!-- The box keeps room for its whole placeholder, and in a column too
  narrow for both the button moves under it rather than cutting the
  words that say what the box is for. A form, so Enter in the box is the
  same request as the button. -->
  <form
    class="mt-base flex flex-wrap items-center gap-snug"
    onsubmit={(event) => {
      event.preventDefault();
      talkIn();
    }}
  >
    <div class="min-w-[min(100%,26ch)] flex-1">
      <Field
        label={say($lang, "bld_room_name")}
        labelling="hidden"
        placeholder={say($lang, "bld_room_name")}
        value={roomName}
        onInput={(value) => {
          roomName = value;
        }}
      />
    </div>
    <Button
      label={say($lang, "bld_room_talk")}
      type="submit"
      tone={Option.isSome(named) ? "primary" : "secondary"}
    />
  </form>
</div>
