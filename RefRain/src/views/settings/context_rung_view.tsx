// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Show, untrack } from "solid-js";

import { readAnswer } from "../../core/answered";
import { configureContext } from "../../core/commands";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, AxError, IdemKey, Query } from "../../wire";
import Field from "../parts/field.solid";
import Unanswered from "../parts/unanswered.solid";
import { HALL } from "../shared/buildings";
import BuildingPicker from "./building_picker";
import SaveCard from "./save_card_view";
import { edited, RECEIPT_MS, sent, waited, type Saving } from "./saving";

interface Write {
  readonly idem: IdemKey;
  readonly percent: number;
  readonly started: number;
}

interface Change {
  readonly typed: string;
  readonly write?: Write;
  readonly error?: AxError;
  readonly uncertain?: boolean;
}

export default function ContextRung() {
  const u = ui();
  const lang = storeValue(u.lang);
  const receipts = storeValue(u.conn.receipts);
  const [at, setAt] = createSignal(HALL);
  const [drafts, setDrafts] = createSignal<ReadonlyMap<Address, Change>>(new Map());
  const [saved, setSaved] = createSignal<Address>();
  const query = createMemo((): Query => ({ config: { addr: at() } }));
  const answer = storeValue(() => u.conn.asking.ask(query()));
  const read = createMemo(() =>
    readAnswer(answer(), (frame) => ("config" in frame ? frame.config.second : undefined)),
  );
  const settled = createMemo(() => {
    const result = read();
    return result.kind === "held" ? result.value : undefined;
  });
  const onDisk = () => {
    const held = settled();
    return held === undefined || held.from === "default" ? "" : String(held.percent);
  };
  const change = () => drafts().get(at());
  const operation = () => change()?.write;
  const box = () => change()?.typed ?? onDisk();
  const word = (key: Key) => say(lang(), key);
  const refresh = (): void => u.conn.asking.refresh(query());
  const range = () => {
    const domain = settled()?.domain;
    return domain === undefined
      ? undefined
      : fill(word("context_second_range"), {
          min: String(domain.min),
          max: String(domain.max),
        });
  };
  const invalid = () => {
    const domain = settled()?.domain;
    const number = Number(box());
    return (
      domain === undefined ||
      box() === "" ||
      !Number.isInteger(number) ||
      number < domain.min ||
      number > domain.max
    );
  };

  function keep(addr: Address, value?: Change): void {
    setDrafts((before) => {
      const next = new Map(before);
      if (value === undefined) next.delete(addr);
      else next.set(addr, value);
      return next;
    });
  }

  function edit(typed: string): void {
    setSaved(undefined);
    const current = change();
    if (current?.write !== undefined) keep(at(), { ...current, typed });
    else keep(at(), typed === onDisk() ? undefined : { typed });
  }

  function save(): void {
    const current = change();
    if (current === undefined || current.write !== undefined || invalid()) return;
    const percent = Number(current.typed);
    const command = configureContext(at(), percent);
    if (!("configure_building" in command) || !u.send(command)) return;
    keep(at(), {
      typed: current.typed,
      write: { idem: command.configure_building.idem, percent, started: u.now() },
    });
    refresh();
  }

  createEffect(
    () => ({ addr: at(), established: settled(), write: operation(), receipts: receipts() }),
    ({ addr, established, write, receipts }) => {
      if (write === undefined) return;
      const current = untrack(drafts).get(addr);
      if (current?.write !== write) return;
      const outcome = receipts.get(write.idem);
      if (outcome?.kind === "refused") {
        keep(addr, { typed: current.typed, error: outcome.error });
        return;
      }
      if (
        established !== undefined &&
        established.from !== "default" &&
        established.percent === write.percent
      ) {
        setSaved(addr);
        keep(addr, current.typed === String(write.percent) ? undefined : { typed: current.typed });
      }
    },
  );

  createEffect(
    () => ({ addr: at(), question: query(), write: operation() }),
    ({ addr, question, write }) => {
      if (write === undefined) return;
      const timer = window.setInterval(() => u.conn.asking.refresh(question), 1_000);
      const expiry = window.setTimeout(
        () => {
          const current = drafts().get(addr);
          if (current?.write === write) keep(addr, { ...current, uncertain: true });
          window.clearInterval(timer);
        },
        Math.max(0, RECEIPT_MS - (u.now() - write.started)),
      );
      return () => {
        window.clearInterval(timer);
        window.clearTimeout(expiry);
      };
    },
  );

  const status = createMemo<Saving>(() => {
    const current = change();
    if (current?.write !== undefined) {
      const pending = sent(onDisk());
      return current.uncertain === true ? waited(pending) : pending;
    }
    if (current?.error !== undefined) return { kind: "refused", error: current.error };
    return saved() === at() && current === undefined
      ? { kind: "saved" }
      : edited(current !== undefined);
  });
  const unavailable = () => {
    const reading = read();
    return reading.kind === "unavailable" ? reading : undefined;
  };

  return (
    <SaveCard
      title={word("context_second")}
      note={word("context_second_note")}
      settled={word("settings_next_run")}
      saveLabel={word("context_second_save")}
      saving={status()}
      onSave={save}
      onCheck={refresh}
      blocked={status().kind !== "unverified" && invalid() ? "context_second_unchanged" : undefined}
    >
      <BuildingPicker label={word("context_second_building")} value={at()} onPick={setAt} />
      <Field
        label={word("context_second")}
        labelling="hidden"
        kind="number"
        step={1}
        suffix={word("context_second_unit")}
        mono
        {...(settled()?.from === "default" ? { placeholder: String(settled()?.percent) } : {})}
        {...(change() !== undefined && box() !== "" && invalid()
          ? { error: range() ?? word("context_second_unchanged") }
          : {})}
        value={box()}
        onInput={edit}
      />
      <Show when={range()}>
        <span class="min-w-0 text-note text-text-faint">{range()}</span>
      </Show>
      <Show when={unavailable()}>
        {(result) => <Unanswered query={result().query} asked={query()} reason={result().reason} />}
      </Show>
    </SaveCard>
  );
}
