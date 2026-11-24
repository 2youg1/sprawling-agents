// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { derived, get, writable } from "../../core/store";
import { Option, Schema } from "effect";
import { createEffect } from "solid-js";

import { dispatch, steer } from "../../core/commands";
import type { Dispatch } from "../../core/commands";
import type { Connection } from "../../core/socket";
import { storeValue } from "../../core/solid_store";
import { Command, IdemKey } from "../../wire";
import type { Address, AxError, RunId, Seq } from "../../wire";
import { editStoredOperation, readStoredOperation } from "./attachment_storage";
import { attachmentsAt, loadAttachments, removeAttachments } from "./attachments";
import { draftPlace } from "./draft";
import { withSources } from "./source_context";

interface Flight {
  readonly command: Command;
  readonly key: IdemKey;
  readonly words: string;
  readonly attachments: readonly string[];
  readonly beganAt: number;
}

type SpeechDoor = Pick<Connection, "state" | "receipts" | "command">;

export type SpeechState =
  | { readonly kind: "idle" }
  | { readonly kind: "preparing" }
  | { readonly kind: "pending" | "unknown"; readonly flight: Flight }
  | { readonly kind: "refused"; readonly error: AxError };

const empty: SpeechState = { kind: "idle" };
const RECEIPT_MS = 10_000;
const byDraft = writable<ReadonlyMap<string, SpeechState>>(new Map());
const loading = new Map<string, Promise<void>>();
const readFlight = Schema.decodeUnknownOption(
  Schema.Struct({
    command: Command,
    key: IdemKey,
    words: Schema.String,
    attachments: Schema.Array(Schema.String),
    beganAt: Schema.Number,
  }),
);

export function loadSpeech(key: string): Promise<void> {
  const underway = loading.get(key);
  if (underway !== undefined) return underway;
  const reading = readStoredOperation(key).then((stored) => {
    const flight = Option.getOrNull(readFlight(stored));
    if (
      flight !== null &&
      keyOf(flight.command) === flight.key &&
      get(byDraft).get(key) === undefined
    ) {
      set(key, { kind: "unknown", flight });
    }
  });
  loading.set(key, reading);
  return reading;
}

function set(key: string, value: SpeechState): void {
  byDraft.update((current) => {
    const next = new Map(current);
    if (value.kind === "idle") next.delete(key);
    else next.set(key, value);
    return next;
  });
}

export function speechAt(draft: string | undefined) {
  return derived(byDraft, ($held): SpeechState =>
    draft === undefined ? empty : ($held.get(draft) ?? empty),
  );
}

export function watchSpeech(
  conn: SpeechDoor,
  draft: () => string | undefined,
  state: () => SpeechState,
  received: (draft: string, words: string) => void,
): void {
  const receipts = storeValue(conn.receipts);
  const connection = storeValue(conn.state);
  createEffect(draft, (place) => {
    if (place !== undefined) void Promise.all([loadAttachments(place), loadSpeech(place)]);
  });
  function reconcile(place: string): void {
    const words = reconcileSpeech(conn, place);
    if (words !== null) received(place, words);
  }
  createEffect(
    () => {
      const held = state();
      return "flight" in held && (receipts().has(held.flight.key) || connection().kind !== "live")
        ? draft()
        : undefined;
    },
    (place) => {
      if (place !== undefined) reconcile(place);
    },
  );
  createEffect(
    () => {
      const held = state();
      const place = draft();
      return held.kind === "pending" && place !== undefined
        ? { place, at: held.flight.beganAt }
        : undefined;
    },
    (pending) => {
      if (pending === undefined) return;
      const timer = setTimeout(
        () => reconcile(pending.place),
        Math.max(0, RECEIPT_MS - (Date.now() - pending.at)),
      );
      return () => clearTimeout(timer);
    },
  );
}

export interface SpeechTarget {
  readonly room: Address;
  readonly session: Seq | null;
  readonly run: RunId | undefined;
  readonly historical?: boolean;
  readonly imageInput?: boolean | undefined;
}

interface DispatchChoice extends Pick<Dispatch, "effort" | "policy" | "goal"> {
  readonly onDispatch?: ((task: string) => void) | undefined;
}

export type SpeechResult = "sent" | "offline" | "image-unavailable" | "image-failed" | "busy";

function keyOf(command: Command): IdemKey | null {
  return "dispatch" in command
    ? command.dispatch.idem
    : "steer" in command
      ? command.steer.idem
      : null;
}

const MAX_CROPS = 4;
const MAX_PNG_BYTES = 1_572_864;

function encodeImage(blob: Blob): Promise<string | null> {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onloadend = () => {
      const prefix = "data:image/png;base64,";
      const value = reader.result;
      resolve(
        typeof value === "string" && value.startsWith(prefix) ? value.slice(prefix.length) : null,
      );
    };
    reader.readAsDataURL(blob);
  });
}

export async function submitSpeech(
  conn: SpeechDoor,
  target: SpeechTarget,
  words: string,
  choice: DispatchChoice,
): Promise<SpeechResult> {
  const draft = draftPlace(target.room, target.session);
  await Promise.all([loadAttachments(draft), loadSpeech(draft)]);
  const state = get(byDraft).get(draft);
  if (state?.kind === "pending" || state?.kind === "unknown" || state?.kind === "preparing")
    return "busy";
  const attachments = get(attachmentsAt(draft));
  if (get(conn.state).kind !== "live") return "offline";
  const crops = attachments.flatMap(({ selection }) =>
    selection.kind === "pdf-region" ? [selection.image] : [],
  );
  if (crops.length > 0 && target.imageInput === false) return "image-unavailable";
  if (crops.length > MAX_CROPS || crops.some((crop) => crop.size > MAX_PNG_BYTES))
    return "image-failed";
  set(draft, { kind: "preparing" });
  const encoded = crops.length === 0 ? [] : await Promise.all(crops.map(encodeImage));
  const images = encoded.filter((image): image is string => image !== null);
  if (images.length !== encoded.length) {
    set(draft, empty);
    return "image-failed";
  }
  if (get(conn.state).kind !== "live") {
    set(draft, empty);
    return "offline";
  }
  const task = withSources(words, attachments);
  const { onDispatch, ...options } = choice;
  const command =
    target.run === undefined
      ? dispatch({
          addr: target.room,
          task,
          resume: target.historical === true ? target.session : null,
          images,
          ...options,
        })
      : steer(target.run, task, images);
  const key = keyOf(command);
  if (key === null) {
    set(draft, empty);
    return "offline";
  }
  const flight = {
    command,
    key,
    words,
    attachments: attachments.map(({ id }) => id),
    beganAt: Date.now(),
  };
  await editStoredOperation(draft, flight);
  if (get(conn.state).kind !== "live") {
    void editStoredOperation(draft, null);
    set(draft, empty);
    return "offline";
  }
  if (!conn.command(command)) {
    void editStoredOperation(draft, null);
    set(draft, empty);
    return "offline";
  }
  if ("dispatch" in command) onDispatch?.(task);
  set(draft, { kind: "pending", flight });
  return "sent";
}

export function reconcileSpeech(conn: SpeechDoor, draft: string): string | null {
  const state = get(byDraft).get(draft);
  if (state?.kind !== "pending" && state?.kind !== "unknown") return null;
  const receipt = get(conn.receipts).get(state.flight.key);
  if (receipt?.kind === "accepted") {
    removeAttachments(draft, state.flight.attachments);
    set(draft, empty);
    void editStoredOperation(draft, null);
    return state.flight.words;
  }
  if (receipt?.kind === "refused") {
    set(draft, { kind: "refused", error: receipt.error });
    void editStoredOperation(draft, null);
    return null;
  }
  if (
    state.kind === "pending" &&
    (get(conn.state).kind !== "live" || Date.now() - state.flight.beganAt >= RECEIPT_MS)
  ) {
    set(draft, { kind: "unknown", flight: state.flight });
  }
  return null;
}

export function recheckSpeech(conn: SpeechDoor, draft: string): void {
  const state = get(byDraft).get(draft);
  if (state?.kind !== "unknown" || get(conn.state).kind !== "live") return;
  if (conn.command(state.flight.command)) {
    set(draft, { kind: "pending", flight: { ...state.flight, beganAt: Date.now() } });
  }
}
