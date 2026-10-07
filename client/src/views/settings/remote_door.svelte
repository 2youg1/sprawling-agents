<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The door controls of the remote group (client/Spec.lean §4-57, the
  // model in client/spec/Views/Door.lean): open the door for a chosen
  // time, close it, replace the city key, and the code input the city's
  // answer opens. `step` in `remote_door.ts` decides every move; this
  // component sends what it says to send, feeds it the city's answers
  // and puts the focus where it says. Each control carries a glyph, a
  // visible name and a note on hover and keyboard focus (D55), drawn by
  // `door_key.look.svelte` from the `DoorKeyLook` this file builds.

  import { tick } from "svelte";

  import { fill, say, type Key } from "../../core/lang";
  import { ui } from "../../ui";
  import type { GlyphName } from "../parts/glyph";
  import Segmented from "../parts/segmented.svelte";
  import DoorKey from "./door_key.look.svelte";
  import { RECEIPT_MS } from "./saving";
  import {
    IDLE,
    LASTINGS,
    LASTING_DEFAULT,
    answerOf,
    doorCommand,
    refusedKey,
    step,
    type Door,
    type DoorEvent,
    type DoorKeyLook,
    type DoorSent,
  } from "./remote_door";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const uid = $props.id();

  let door = $state.raw<Door>(IDLE);
  let lasting = $state(LASTING_DEFAULT);
  // What the status line says after the last move that left the machine.
  let told = $state<{ readonly key: Key; readonly code: string } | null>(null);
  let openButton = $state<HTMLElement | undefined>(undefined);
  let keyButton = $state<HTMLElement | undefined>(undefined);
  let input = $state<HTMLInputElement | undefined>(undefined);
  let patience: ReturnType<typeof setTimeout> | undefined = undefined;

  // A refusal that arrives while a request or a code is in flight is
  // the door's answer; `step` drops one that arrives at any other time.
  let seen = $belief.refusal;
  $effect(() => {
    const error = $belief.refusal;
    if (error === null || error === seen) return;
    seen = error;
    move(answerOf(error.code));
  });

  // One event through the machine: the command it sends goes out, the
  // focus goes where the new door says, and the status line follows.
  function move(event: DoorEvent): void {
    const [next, sent] = step(door, event);
    const before = door;
    door = next;
    if (patience !== undefined && next.phase.kind !== "confirming") {
      clearTimeout(patience);
      patience = undefined;
    }
    if (sent !== null && !send(sent)) {
      // Nothing left the page: the machine hears `unsent`, not a refusal
      // the city never sent, and the line says why in the page's words.
      move({ kind: "unsent" });
      told = { key: "remote_door_unsent", code: "" };
      return;
    }
    told = toldOf(before, next);
    if (next.focus !== before.focus) void tick().then(() => {
        focus(next);
      });
  }

  // Whether the command left the page; a closed link sends nothing.
  function send(sent: DoorSent): boolean {
    if (!u.send(doorCommand(sent, lastingMs()))) return false;
    if (sent.kind === "confirm") patience = setTimeout(() => {
        move({ kind: "settled" });
      }, RECEIPT_MS);
    return true;
  }

  function lastingMs(): number {
    return LASTINGS.find(([spelling]) => spelling === lasting)?.[1] ?? 0;
  }

  function toldOf(before: Door, next: Door): { readonly key: Key; readonly code: string } | null {
    if (next.phase.kind === "refused") return { key: refusedKey(next.phase.code), code: next.phase.code };
    if (before.phase.kind === "confirming" && next.phase.kind === "idle")
      return { key: before.phase.opener === "door" ? "remote_door_done_door" : "remote_door_done_key", code: "" };
    return next.phase.kind === "idle" ? told : null;
  }

  function focus(now: Door): void {
    switch (now.focus.kind) {
      case "elsewhere":
        return;
      case "input":
        input?.focus();
        return;
      case "opener":
        (now.focus.opener === "door" ? openButton : keyButton)?.querySelector("button")?.focus();
    }
  }

  function close(): void {
    told = u.send(doorCommand("close", 0)) ? { key: "remote_door_closing", code: "" } : { key: "remote_door_unsent", code: "" };
  }

  const asking = $derived(door.phase.kind === "awaiting" || door.phase.kind === "confirming");
  const busy = $derived(door.phase.kind === "requesting" || door.phase.kind === "confirming");

  // One door control; a press on a disabled one is dropped here.
  function doorKey(glyph: GlyphName, name: Key, note: Key, disabled: boolean, onPress: () => void): DoorKeyLook {
    return {
      glyph,
      label: say($lang, name),
      note: say($lang, note),
      wire: {
        type: "button",
        "aria-disabled": disabled,
        onclick: () => {
          if (!disabled) onPress();
        },
      },
    };
  }
</script>

<section class="flex flex-col gap-base" aria-labelledby={`${uid}-door`}>
  <h3 id={`${uid}-door`} class="text-label font-label text-text">{say($lang, "remote_door")}</h3>
  <p class="text-body text-text-quiet">{say($lang, "remote_door_about")}</p>
  <div class="flex flex-col gap-tight">
    <span class="text-note text-text-quiet">{say($lang, "remote_door_lasting")}</span>
    <Segmented
      label={say($lang, "remote_door_lasting")}
      tone="accent"
      options={LASTINGS.map(([spelling]) => ({ value: spelling, label: spelling }))}
      held={lasting}
      onPick={(next: string) => {
        lasting = next;
      }}
    />
  </div>
  <div class="flex flex-wrap gap-snug">
    <span bind:this={openButton} class="contents">
      <DoorKey
        {...doorKey("door", "remote_door_open", "remote_door_open_note", busy || asking, () => {
          move({ kind: "press", opener: "door" });
        })}
      />
    </span>
    <DoorKey {...doorKey("gate", "remote_door_close", "remote_door_close_note", false, close)} />
    <span bind:this={keyButton} class="contents">
      <DoorKey
        {...doorKey("key", "remote_key_replace", "remote_key_replace_note", busy || asking, () => {
          move({ kind: "press", opener: "key" });
        })}
      />
    </span>
  </div>
  <p class="text-note text-text-faint">{say($lang, "remote_key_where")}</p>
  {#if asking}
    <form
      class="flex flex-wrap items-end gap-snug"
      onsubmit={(event) => {
        event.preventDefault();
        move({ kind: "submit" });
      }}
    >
      <div class="flex min-w-0 flex-col gap-tight">
        <label class="text-note text-text-quiet" for={`${uid}-code`}>{say($lang, "remote_door_code")}</label>
        <input
          id={`${uid}-code`}
          bind:this={input}
          class="h-control w-code min-w-0 rounded-control border border-edge-input bg-raised px-base font-mono text-body text-text"
          autocomplete="off"
          spellcheck="false"
          aria-describedby={`${uid}-code-help`}
          readonly={door.phase.kind === "confirming"}
          value={door.typed}
          oninput={(event) => {
            move({ kind: "type", text: event.currentTarget.value });
          }}
          onkeydown={(event) => {
            if (event.key !== "Escape") return;
            event.preventDefault();
            move({ kind: "escape" });
          }}
        />
      </div>
      <DoorKey
        glyph="check"
        label={say($lang, "remote_door_confirm")}
        note={say($lang, "remote_door_confirm_note")}
        wire={{ type: "submit", "aria-disabled": door.phase.kind === "confirming" }}
      />
      <p id={`${uid}-code-help`} class="basis-full text-note text-text-faint">{say($lang, "remote_door_code_help")}</p>
    </form>
  {/if}
  <p role="status" class="text-body text-text-quiet empty:hidden">{told === null ? "" : fill(say($lang, told.key), { code: told.code })}</p>
</section>
