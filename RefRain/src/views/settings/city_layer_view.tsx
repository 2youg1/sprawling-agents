// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal, Show } from "solid-js";

import { configureCity, EFFORTS } from "../../core/commands";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { AxError, Effort, IdemKey, KeepWarm } from "../../wire";
import { DiagnosticView } from "../diagnostic_view";
import Button from "../parts/button.solid";
import Segmented from "../parts/segmented.solid";
import Unanswered from "../parts/unanswered.solid";
import { HALL } from "../shared/buildings";
import { RECEIPT_MS } from "./saving";

const KEEP_WARM: readonly KeepWarm[] = ["off", "five_minute"];
const WARM_WORD: Record<KeepWarm, Key> = {
  off: "city_keep_warm_off",
  five_minute: "city_keep_warm_five_minute",
};

type Change =
  | { readonly field: "warm"; readonly value: KeepWarm }
  | { readonly field: "effort"; readonly value: Effort };
type Write = Change & { readonly idem: IdemKey };
type State =
  | { readonly kind: "idle" | "saved" | "unsent" }
  | { readonly kind: "pending" | "unknown"; readonly operation: Write }
  | { readonly kind: "refused"; readonly error: AxError };

export default function CityLayer() {
  const u = ui();
  const lang = storeValue(u.lang);
  const receipts = storeValue(u.conn.receipts);
  const query = { config: { addr: HALL } };
  const config = storeValue(u.conn.asking.ask(query));
  const [state, setState] = createSignal<State>({ kind: "idle" });
  const settings = () => {
    const seen = config();
    return seen !== undefined && "config" in seen ? seen.config : undefined;
  };
  const unavailable = () => {
    const seen = config();
    return seen !== undefined && "unavailable" in seen ? seen.unavailable : undefined;
  };
  const effort = () => settings()?.city_effort ?? null;
  const warm = () => (settings() === undefined ? null : (settings()?.city_keep_warm ?? "off"));
  const word = (key: Key) => say(lang(), key);
  const check = (): void => u.conn.asking.refresh(query);
  const pending = () => {
    const current = state();
    return current.kind === "pending" || current.kind === "unknown" ? current.operation : undefined;
  };
  const reason = (): { why?: string } => {
    if (state().kind === "pending") return { why: word("saving_sent") };
    if (state().kind === "unknown") return { why: word("saving_unverified") };
    return {};
  };

  function save(change: Change): void {
    if (settings() === undefined || pending() !== undefined) return;
    const command =
      change.field === "warm"
        ? configureCity(change.value, null)
        : configureCity(null, change.value);
    if (!("configure_city" in command) || !u.send(command)) {
      setState({ kind: "unsent" });
      return;
    }
    setState({ kind: "pending", operation: { ...change, idem: command.configure_city.idem } });
    check();
  }

  createEffect(
    () => ({ operation: pending(), config: settings(), receipts: receipts() }),
    ({ operation, config, receipts }) => {
      if (operation === undefined) return;
      const reply = receipts.get(operation.idem);
      if (reply?.kind === "refused") {
        setState({ kind: "refused", error: reply.error });
        return;
      }
      if (
        config !== undefined &&
        (operation.field === "warm"
          ? config.city_keep_warm === operation.value
          : config.city_effort === operation.value)
      )
        setState({ kind: "saved" });
    },
  );

  createEffect(pending, (operation) => {
    if (operation === undefined) return;
    const poll = window.setInterval(check, 1_000);
    const expiry = window.setTimeout(() => {
      setState({ kind: "unknown", operation });
      window.clearInterval(poll);
    }, RECEIPT_MS);
    return () => {
      window.clearTimeout(expiry);
      window.clearInterval(poll);
    };
  });

  function status(): string {
    switch (state().kind) {
      case "idle":
        return "";
      case "unsent":
        return word("setup_account_not_sent");
      case "pending":
        return word("saving_sent");
      case "unknown":
        return word("saving_unverified");
      case "saved":
        return word("settings_next_run");
      case "refused":
        return word("saving_refused");
    }
  }
  const error = () => {
    const held = state();
    return held.kind === "refused" ? held.error : undefined;
  };

  return (
    <section
      class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug"
      aria-label={word("city_layer")}
    >
      <div class="flex flex-col gap-hair">
        <span class="text-label font-label text-text">{word("city_layer")}</span>
        <p class="text-note text-text-faint">{word("city_layer_note")}</p>
      </div>
      <Show when={settings() !== undefined}>
        <div class="flex flex-col gap-tight">
          <span class="text-note text-text-quiet">{word("city_effort")}</span>
          <Segmented
            label={word("city_effort")}
            tone="accent"
            options={EFFORTS.map((value) => ({
              value,
              label: word(`effort_${value}`),
              ...reason(),
            }))}
            held={effort()}
            onPick={(value) => save({ field: "effort", value })}
          />
        </div>
        <div class="flex flex-col gap-tight">
          <span class="text-note text-text-quiet">{word("city_keep_warm")}</span>
          <Segmented
            label={word("city_keep_warm")}
            options={KEEP_WARM.map((value) => ({
              value,
              label: word(WARM_WORD[value]),
              ...reason(),
            }))}
            held={warm()}
            onPick={(value) => save({ field: "warm", value })}
          />
          <span class="text-note text-text-faint">{word("city_keep_warm_note")}</span>
        </div>
      </Show>
      <Show when={unavailable()}>
        {(lost) => <Unanswered query={lost().query} asked={query} reason={lost().reason} />}
      </Show>
      <p role="status" class="text-note text-text-quiet empty:hidden">
        {status()}
      </p>
      <Show when={state().kind === "unknown"}>
        <Button label={word("saving_check")} result="unknown" onPress={check} />
      </Show>
      <Show when={error()}>{(failure) => <DiagnosticView lang={lang()} error={failure()} />}</Show>
    </section>
  );
}
