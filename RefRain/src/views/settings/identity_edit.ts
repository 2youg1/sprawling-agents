// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal, type Accessor } from "solid-js";

import { QUERIES } from "../../core/asking";
import { putIdentity } from "../../core/commands";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { AxError, IdemKey, IdentityCard } from "../../wire";
import { edited, RECEIPT_MS, sent, waited, type Saving } from "./saving";

interface IdentityValue<T> {
  readonly text: T;
  readonly base: string;
}

interface Write<T> {
  readonly idem: IdemKey;
  readonly text: T;
}

interface IdentityChange<T> {
  readonly text: T;
  readonly base: string;
  readonly write?: Write<T>;
  readonly unknown?: boolean;
  readonly error?: AxError;
}

export function identityEditor<T>(
  read: Accessor<IdentityValue<T> | undefined>,
  card: (text: T) => IdentityCard,
  equal: (a: T, b: T) => boolean,
) {
  const u = ui();
  const receipts = storeValue(u.conn.receipts);
  const [change, setChange] = createSignal<IdentityChange<T>>();
  const [saved, setSaved] = createSignal(false);
  const actual = () => change()?.text ?? read()?.text;
  const check = () => u.conn.asking.refresh(QUERIES.identity);
  const active = () => change()?.write;
  const conflict = () => change()?.error?.code === "E_VERSION_CONFLICT";

  function edit(text: T): void {
    const held = change();
    const base = held?.base ?? read()?.base;
    if (base === undefined) return;
    setSaved(false);
    setChange(held?.write !== undefined || conflict() ? { ...held, base, text } : { base, text });
  }

  function rebase(): void {
    const next = read();
    const local = change();
    if (next === undefined || local === undefined || !conflict()) return;
    setChange(equal(next.text, local.text) ? undefined : { base: next.base, text: local.text });
  }

  function save(): void {
    const local = change();
    const current = read();
    if (local === undefined || current === undefined || local.write !== undefined || conflict())
      return;
    if (equal(local.text, current.text)) {
      setChange(undefined);
      return;
    }
    const command = putIdentity(card(local.text), local.base);
    if (!("put_identity" in command) || !u.send(command)) return;
    setChange({ ...local, write: { idem: command.put_identity.idem, text: local.text } });
    check();
  }

  createEffect(
    () => ({ local: change(), current: read(), receipts: receipts() }),
    ({ local, current, receipts }) => {
      const write = local?.write;
      if (write === undefined || local === undefined) return;
      const outcome = receipts.get(write.idem);
      if (outcome?.kind === "refused") {
        setChange({ base: local.base, text: local.text, error: outcome.error });
        check();
        return;
      }
      if (current !== undefined && equal(current.text, write.text)) {
        setSaved(true);
        setChange(
          equal(current.text, local.text) ? undefined : { base: current.base, text: local.text },
        );
      }
    },
  );

  createEffect(active, (write) => {
    if (write === undefined) return;
    const poll = window.setInterval(check, 1_000);
    const timer = window.setTimeout(() => {
      const local = change();
      if (local?.write === write) setChange({ ...local, unknown: true });
      window.clearInterval(poll);
    }, RECEIPT_MS);
    return () => {
      window.clearTimeout(timer);
      window.clearInterval(poll);
    };
  });

  const status = (): Saving => {
    const local = change();
    if (local === undefined) return saved() ? { kind: "saved" } : edited(false);
    if (local.error !== undefined) return { kind: "refused", error: local.error };
    if (local.write !== undefined) {
      const pending = sent(local.base);
      return local.unknown === true ? waited(pending) : pending;
    }
    const current = read();
    return edited(current !== undefined && !equal(local.text, current.text));
  };

  return { actual, check, conflict, edit, rebase, save, status };
}
