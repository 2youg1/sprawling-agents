// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal, createUniqueId, onCleanup, Show, untrack } from "solid-js";

import { storeValue } from "../../core/solid_store";

import { fill, say, type Key } from "../../core/lang";
import type { AxCode, IdemKey } from "../../wire";
import { ui } from "../../ui";
import type { GlyphName } from "../parts/glyph";
import Button from "../parts/button.solid";
import Segmented from "../parts/segmented.solid";
import Glyph from "../parts/glyph.solid";
import ControlTip from "../parts/control_tip";
import { RECEIPT_MS } from "./saving";
import {
  IDLE,
  LASTINGS,
  LASTING_DEFAULT,
  doorCommand,
  receiptOf,
  requestOf,
  refusedKey,
  step,
  type Door,
  type DoorEvent,
  type DoorKeyLook,
  type DoorSent,
} from "./remote_door";

export function DoorKey(props: DoorKeyLook) {
  const hint = createUniqueId();
  return (
    <ControlTip id={hint} text={props.note}>
      <button
        type={props.wire.type}
        aria-disabled={props.wire["aria-disabled"] ? "true" : "false"}
        aria-describedby={hint}
        data-control="surface"
        class="relative flex h-control shrink-0 items-center gap-tight rounded-control bg-raised px-base text-label text-text aria-disabled:bg-raised aria-disabled:text-text-disabled"
        onClick={(event) => {
          if (props.wire["aria-disabled"]) event.preventDefault();
          else props.wire.onclick?.();
        }}
      >
        <Glyph name={props.glyph} size="sm" class="shrink-0" />
        {props.label}
      </button>
    </ControlTip>
  );
}

export default function RemoteDoor() {
  const u = ui();
  const lang = storeValue(u.lang);
  const receipts = storeValue(u.conn.receipts);
  const connection = storeValue(u.conn.state);
  const uid = createUniqueId();

  interface Status {
    readonly key: Key;
    readonly code: string;
  }
  type Closure =
    | { readonly kind: "waiting" | "unknown"; readonly request: IdemKey }
    | { readonly kind: "done" | "unsent" }
    | { readonly kind: "refused"; readonly code: AxCode };

  const [door, setDoor] = createSignal<Door>(IDLE);
  const [lasting, setLasting] = createSignal(LASTING_DEFAULT);
  const [told, setTold] = createSignal<Status | null>(null);
  const [closing, setClosing] = createSignal<Closure | null>(null);
  const [tracked, setTracked] = createSignal<IdemKey | null>(null);
  let approval: IdemKey | null = null;
  let openButton: HTMLElement | undefined;
  let keyButton: HTMLElement | undefined;
  let checkButton: HTMLElement | undefined;
  let closeCheck: HTMLElement | undefined;
  let closeButton: HTMLElement | undefined;
  let input: HTMLInputElement | undefined;
  let confirmation: HTMLFormElement | undefined;
  let patience: ReturnType<typeof setTimeout> | undefined;
  let closingPatience: ReturnType<typeof setTimeout> | undefined;

  onCleanup(() => {
    clearTimeout(patience);
    clearTimeout(closingPatience);
  });

  createEffect(
    () => ({ receipt: receipts(), request: tracked(), close: closing() }),
    ({ receipt, request, close }) => {
      const reply = receiptOf(request, receipt);
      if (reply !== null) move(reply);
      if (close?.kind !== "waiting" && close?.kind !== "unknown") return;
      const closed = receiptOf(close.request, receipt);
      if (closed?.kind === "settled" || closed?.kind === "refusal") {
        clearTimeout(closingPatience);
        if (document.activeElement === closeCheck?.querySelector("button"))
          closeButton?.querySelector("button")?.focus();
        setClosing(
          closed.kind === "settled" ? { kind: "done" } : { kind: "refused", code: closed.code },
        );
      }
    },
  );

  createEffect(connection, (state) => {
    if (state.kind === "live") return;
    untrack(() => {
      if (tracked() !== null) move({ kind: "timed_out" });
      const waiting = closing();
      if (waiting?.kind === "waiting") closingUnknown(waiting.request);
    });
  });

  function move(event: DoorEvent): void {
    const before = door();
    const [next, sent] = step(before, event);
    if (next === before) return;
    const here = document.activeElement;
    const checking = here === checkButton?.querySelector("button");
    const mayRefocus =
      here === document.body ||
      confirmation?.contains(here) === true ||
      here === openButton?.querySelector("button") ||
      here === keyButton?.querySelector("button") ||
      checking;
    setDoor(next);
    if (event.kind === "pending") approval = tracked();
    clearTimeout(patience);
    if (sent !== null && !send(sent)) {
      move({ kind: "unsent" });
      return;
    }
    if (
      next.phase.kind !== "requesting" &&
      next.phase.kind !== "confirming" &&
      next.phase.kind !== "unknown"
    )
      setTracked(null);
    setTold(toldOf(before, next, event));
    if (mayRefocus && (next.focus.kind !== before.focus.kind || checking))
      requestAnimationFrame(() => {
        focus(next);
      });
  }

  function send(sent: DoorSent): boolean {
    const command = doorCommand(sent, lastingMs(), approval);
    if (command === null) return false;
    const request = requestOf(command);
    if (request === null || !u.send(command)) return false;
    setTracked(request);
    patience = setTimeout(() => {
      if (tracked() === request) move({ kind: "timed_out" });
    }, RECEIPT_MS);
    return true;
  }

  function lastingMs(): number {
    return LASTINGS.find(([spelling]) => spelling === lasting())?.[1] ?? 0;
  }

  function toldOf(before: Door, next: Door, event: DoorEvent): Status | null {
    if (event.kind === "unsent") return { key: "remote_door_unsent", code: "" };
    if (event.kind === "settled" && before.phase.kind !== "idle" && next.phase.kind === "idle") {
      return {
        key: before.phase.opener === "door" ? "remote_door_done_door" : "remote_door_done_key",
        code: "",
      };
    }
    switch (next.phase.kind) {
      case "refused":
        return { key: refusedKey(next.phase.code), code: next.phase.code };
      case "unknown":
        return { key: "remote_door_outcome_unknown", code: tracked() ?? "" };
      case "requesting":
        return { key: "remote_door_requesting", code: "" };
      case "confirming":
        return { key: "remote_door_confirming", code: "" };
      case "idle":
      case "awaiting":
        return null;
    }
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

  function closingUnknown(request: IdemKey): void {
    const current = closing();
    if (current?.kind !== "waiting" || current.request !== request) return;
    clearTimeout(closingPatience);
    setClosing({ kind: "unknown", request });
  }

  function close(): void {
    const command = doorCommand("close", 0);
    if (command === null) return;
    const request = requestOf(command);
    if (request === null || !u.send(command)) {
      setClosing({ kind: "unsent" });
      return;
    }
    setClosing({ kind: "waiting", request });
    closingPatience = setTimeout(() => {
      closingUnknown(request);
    }, RECEIPT_MS);
  }

  function closeStatus(value: Closure | null): Status | null {
    switch (value?.kind) {
      case "waiting":
        return { key: "remote_door_closing", code: "" };
      case "unknown":
        return { key: "remote_door_outcome_unknown", code: value.request };
      case "done":
        return { key: "remote_door_done_close", code: "" };
      case "unsent":
        return { key: "remote_door_unsent", code: "" };
      case "refused":
        return { key: refusedKey(value.code), code: value.code };
      case undefined:
        return null;
    }
  }

  const closed = () => closeStatus(closing());
  const asking = () => door().phase.kind === "awaiting" || door().phase.kind === "confirming";
  const busy = () => door().phase.kind === "requesting" || door().phase.kind === "confirming";
  const limited = () => busy() || asking() || door().phase.kind === "unknown";
  const closeLimited = () => closing()?.kind === "waiting" || closing()?.kind === "unknown";

  function doorKey(
    glyph: GlyphName,
    name: Key,
    note: Key,
    disabled: boolean,
    onPress: () => void,
  ): DoorKeyLook {
    return {
      glyph,
      label: say(lang(), name),
      note: say(lang(), note),
      wire: {
        type: "button",
        "aria-disabled": disabled,
        onclick: () => {
          if (!disabled) onPress();
        },
      },
    };
  }
  const word = (key: Key) => say(lang(), key);
  const sayStatus = (status: Status | null) =>
    status === null ? "" : fill(word(status.key), { code: status.code });

  return (
    <section class="flex flex-col gap-base" aria-labelledby={`${uid}-door`}>
      <h3 id={`${uid}-door`} class="text-label font-label text-text">
        {word("remote_door")}
      </h3>
      <p class="text-body text-text-quiet">{word("remote_door_about")}</p>
      <div class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{word("remote_door_lasting")}</span>
        <Segmented
          label={word("remote_door_lasting")}
          tone="accent"
          options={LASTINGS.map(([spelling]) => ({ value: spelling, label: spelling }))}
          held={lasting()}
          onPick={setLasting}
        />
      </div>
      <div class="flex flex-wrap gap-snug">
        <span
          ref={(node) => {
            openButton = node;
          }}
          class="contents"
        >
          <DoorKey
            {...doorKey("door", "remote_door_open", "remote_door_open_note", limited(), () =>
              move({ kind: "press", opener: "door" }),
            )}
          />
        </span>
        <span
          ref={(node) => {
            closeButton = node;
          }}
          class="contents"
        >
          <DoorKey
            {...doorKey(
              "gate",
              "remote_door_close",
              "remote_door_close_note",
              closeLimited(),
              close,
            )}
          />
        </span>
        <span
          ref={(node) => {
            keyButton = node;
          }}
          class="contents"
        >
          <DoorKey
            {...doorKey("key", "remote_key_replace", "remote_key_replace_note", limited(), () =>
              move({ kind: "press", opener: "key" }),
            )}
          />
        </span>
      </div>
      <p class="text-note text-text-faint">{word("remote_key_where")}</p>
      <Show when={asking()}>
        <form
          ref={(node) => {
            confirmation = node;
          }}
          class="flex flex-wrap items-end gap-snug"
          onSubmit={(event) => {
            event.preventDefault();
            move({ kind: "submit" });
          }}
        >
          <div class="flex min-w-0 flex-col gap-tight">
            <label class="text-note text-text-quiet" for={`${uid}-code`}>
              {word("remote_door_code")}
            </label>
            <input
              id={`${uid}-code`}
              ref={(node) => {
                input = node;
              }}
              class="h-control w-code min-w-0 rounded-control border border-edge-input bg-raised px-base font-mono text-body text-text"
              autocomplete="off"
              spellcheck="false"
              aria-describedby={`${uid}-code-help`}
              readonly={door().phase.kind === "confirming"}
              value={door().typed}
              onInput={(event) => move({ kind: "type", text: event.currentTarget.value })}
              onKeyDown={(event) => {
                if (event.key !== "Escape" || event.isComposing) return;
                event.preventDefault();
                event.stopPropagation();
                move({ kind: "escape" });
              }}
            />
          </div>
          <DoorKey
            glyph="check"
            label={word("remote_door_confirm")}
            note={word("remote_door_confirm_note")}
            wire={{
              type: "submit",
              "aria-disabled": door().phase.kind === "confirming" || door().typed === "",
            }}
          />
          <p id={`${uid}-code-help`} class="basis-full text-note text-text-faint">
            {word("remote_door_code_help")}
          </p>
        </form>
      </Show>
      <div class="flex flex-col gap-hair">
        <p role="status" aria-atomic="true" class="text-body text-text-quiet">
          {sayStatus(told())}
        </p>
        <Show when={door().phase.kind === "unknown"}>
          <span
            ref={(node) => {
              checkButton = node;
            }}
          >
            <Button
              label={word("remote_door_checked")}
              result="unknown"
              onPress={() => move({ kind: "checked" })}
            />
          </span>
        </Show>
        <p role="status" aria-atomic="true" class="text-body text-text-quiet">
          {sayStatus(closed())}
        </p>
        <Show when={closing()?.kind === "unknown"}>
          <span
            ref={(node) => {
              closeCheck = node;
            }}
          >
            <Button
              label={word("remote_door_checked")}
              result="unknown"
              onPress={() => {
                closeButton?.querySelector("button")?.focus();
                setClosing(null);
              }}
            />
          </span>
        </Show>
      </div>
    </section>
  );
}
