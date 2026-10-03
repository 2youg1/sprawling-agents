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
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";

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
          <button
            type="button"
            class="flex h-step w-full items-center gap-snug rounded-control px-snug text-left leading-none text-text-quiet hover:bg-chrome"
            onclick={() => {
              onPick(room);
            }}
          >
            <span class="min-w-0 flex-1 truncate">{roomOf(room)}</span>
            {#if living(room) > 0}
              <Badge
                text={fill(say($lang, "city_active"), { n: String(living(room)) })}
                weight="live"
                dot
              />
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="text-note text-text-faint">{say($lang, "bld_no_rooms")}</p>
  {/if}
  <!-- The box keeps room for its whole placeholder, and in a column too
  narrow for both the button moves under it rather than cutting the
  words that say what the box is for. -->
  <div class="mt-base flex flex-wrap items-center gap-snug">
    <input
      class="h-control min-w-[min(100%,26ch)] flex-1 rounded-control border border-edge-input bg-raised px-base text-note placeholder:text-text-faint"
      aria-label={say($lang, "bld_room_name")}
      placeholder={say($lang, "bld_room_name")}
      bind:value={roomName}
      onkeydown={(event) => {
        if (event.key === "Enter") talkIn();
      }}
    />
    <Button
      label={say($lang, "bld_room_talk")}
      tone={Option.isSome(named) ? "primary" : "secondary"}
      onPress={talkIn}
    />
  </div>
</div>
