// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { dynamic } from "@solidjs/web";
import { createMemo, createSignal, createUniqueId, Show, untrack } from "solid-js";
import { say, type Key } from "../core/lang";
import type { LocalDoorIssue, Paired, PairWhy } from "../core/local/entering";
import { storeValue } from "../core/solid_store";
import { DiagnosticView } from "./diagnostic_view";
import { codeOf, doorIssue, WHY, type PairingProps } from "./pairing";
import Button from "./parts/button.solid";
import Field from "./parts/field.solid";

export function PairingPending(props: Pick<PairingProps, "lang">) {
  const lang = storeValue(props.lang);
  return (
    <main class="flex min-h-dvh items-center justify-center bg-page px-wide font-sans text-body text-text">
      <p role="status">{say(lang(), "pair_opening")}</p>
    </main>
  );
}

export default function Pairing(props: PairingProps) {
  const uid = createUniqueId();
  const lang = storeValue(props.lang);
  const [code, setCode] = createSignal("");
  const [named, setNamed] = createSignal(untrack(() => props.label));
  const [busy, setBusy] = createSignal<"pair" | "check" | null>(null);
  const [failure, setFailure] = createSignal<Key | undefined>();
  const [why, setWhy] = createSignal<PairWhy>(untrack(() => props.why));
  const [issue, setIssue] = createSignal<LocalDoorIssue | undefined>(untrack(() => props.issue));
  const [check, setCheck] = createSignal<(() => Promise<Paired>) | undefined>(() =>
    untrack(() => props.check),
  );
  const diagnostic = createMemo(() => {
    const problem = issue();
    return problem === undefined ? undefined : doorIssue(lang(), why(), problem);
  });
  const acceptsCode = () => why() !== "pair_unknown" && why() !== "session_pending";
  let input: HTMLInputElement | undefined;
  const page = () => props.seat !== "specimen";
  const Frame = dynamic(() => (page() ? "main" : "div"));
  const Title = dynamic(() => (page() ? "h1" : "h2"));
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    const typed = codeOf(code());
    if (!typed || busy() !== null || !acceptsCode()) return;
    await attempt("pair", () => props.pair(typed, named().trim() || props.label));
  }
  async function attempt(kind: "pair" | "check", open: () => Promise<Paired>): Promise<void> {
    if (busy() !== null) return;
    setBusy(kind);
    setFailure(undefined);
    const result: Paired = await open().catch(() => ({
      kind: "unreachable",
      issue: { phase: kind === "pair" ? "pair" : "session", problem: "network" },
    }));
    setBusy(null);
    switch (result.kind) {
      case "paired":
        props.onPaired(result.credential);
        break;
      case "refused":
        setCode("");
        setIssue(undefined);
        setFailure("recover_e_pairing_refused");
        input?.focus();
        break;
      case "keyless":
        setFailure("pair_keyless");
        break;
      case "session_lost":
        setWhy("session_lost");
        setIssue(undefined);
        setCheck(undefined);
        break;
      case "pair_unknown":
      case "pair_unconfirmed":
      case "session_pending":
      case "unreachable":
        setCode("");
        setWhy(result.kind);
        setIssue(result.issue);
        setCheck(() => result.check);
        break;
    }
  }
  return (
    <Frame
      class={[
        "flex items-center justify-center bg-page px-wide font-sans text-body text-text",
        page() ? "min-h-dvh" : "py-wide",
      ]}
    >
      <form
        class="flex w-full max-w-measure flex-col gap-wide"
        onSubmit={(event) => void submit(event)}
      >
        <div class="flex flex-col gap-snug">
          <Title class="text-title font-title text-text">{say(lang(), "pair_title")}</Title>
          <Show when={WHY[why()]}>
            {(why) => <p class="text-body text-text">{say(lang(), why())}</p>}
          </Show>
          <Show when={diagnostic()}>
            {(error) => (
              <div role="alert">
                <p class="text-note text-alert">{error().recovery}</p>
                <DiagnosticView error={error()} lang={lang()} />
              </div>
            )}
          </Show>
          <Show when={acceptsCode()}>
            <p class="text-body text-text-quiet">{say(lang(), "pair_where")}</p>
          </Show>
        </div>
        <Show when={acceptsCode()}>
          <div class="flex flex-col gap-snug">
            <label for={uid + "-code"} class="text-note text-text-quiet">
              {say(lang(), "pair_code")}
            </label>
            <input
              id={uid + "-code"}
              value={code()}
              ref={(node) => {
                input = node;
              }}
              onInput={(event) => setCode(event.currentTarget.value)}
              autocomplete="one-time-code"
              autocapitalize="none"
              spellcheck={false}
              aria-invalid={failure() === undefined ? "false" : "true"}
              aria-describedby={failure() === undefined ? undefined : uid + "-failed"}
              class="h-[4.5rem] w-full rounded-card border border-edge-input bg-raised px-pane text-center font-mono text-figure tracking-[0.2em] text-text"
            />
            <Show when={failure()}>
              {(key) => (
                <p id={uid + "-failed"} role="alert" class="text-note text-alert">
                  {say(lang(), key())}
                </p>
              )}
            </Show>
          </div>
          <Field
            label={say(lang(), "pair_label")}
            help={say(lang(), "pair_label_note")}
            value={named()}
            onInput={setNamed}
          />
          <div class="flex justify-end">
            <Button
              label={say(lang(), busy() === "pair" ? "pair_pairing" : "pair_action")}
              tone="primary"
              type="submit"
              loading={busy() === "pair"}
            />
          </div>
        </Show>
        <Show when={check()}>
          {(run) => (
            <div class="flex justify-end">
              <Button
                label={say(lang(), busy() === "check" ? "pair_checking" : "pair_check")}
                loading={busy() === "check"}
                onPress={() => void attempt("check", run())}
              />
            </div>
          )}
        </Show>
      </form>
    </Frame>
  );
}
