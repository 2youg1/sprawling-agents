// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Turning a URL and a key into models a run can be given.
//
// **Two boxes and one control are all the form asks for.** A URL, a
// key, and which shape the endpoint answers in; the id and the display
// name are derived from the URL and folded into `advanced`, because
// they are the city's bookkeeping rather than anything the person came
// holding. What each field means is `./draft`; this file is what the
// form does with them.
//
// The key goes to the vault by its own route and comes back as a
// reference. **The reference is held beside the id it was filed under**:
// the reference is derived from the id, so a reference kept alone
// outlives the name that makes it true, and the second provider a person
// enrolled used to inherit the first one's key.

import { Show, createEffect, createMemo, createSignal } from "solid-js";
import { createStore } from "solid-js/store";

import { attachEndpoint, dialectOf, probeEndpoint, selectModel } from "../../../core/commands";
import type { Endpoint } from "../../../core/commands";
import { enrol, keyField, referenceFor, secretFor } from "../../../core/enrol";
import type { Enrolment, StoredKey } from "../../../core/enrol";
import { normalisedFrom } from "../../../core/probed";
import type { Probed } from "../../../core/probed";
import type { AxError, DialectKind } from "../../../wire";
import { useCommand, useSay, useUi } from "../../../ui";
import { Field } from "../../parts/field";
import { Segmented } from "../../parts/segmented";
import { ModelTable } from "../models";
import type { ModelRow } from "../models";
import { AdvancedFields, BASE_URL, ID_SHAPE, endpointOf, freshDraft, hostOf, idOf, wireChoices } from "./draft";
import { Previews, referenceOf } from "./preview";
import { Reachability, Refusal } from "./reach";

export function AttachForm(props: { readonly onAttached?: () => void }) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const [draft, setDraft] = createStore(freshDraft());
  // The reference the vault answered with, and the id it was filed
  // under. Never one without the other.
  const [held, setHeld] = createSignal<StoredKey | null>(null);
  const [note, setNote] = createSignal<string | null>(null);
  const [report, setReport] = createSignal<AxError | null>(null);
  const [looking, setLooking] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [rows, setRows] = createSignal<readonly ModelRow[]>([]);
  const [advanced, setAdvanced] = createSignal(false);
  // What the URL box held before the city answered with another
  // spelling of the same endpoint.
  const [rewritten, setRewritten] = createSignal<string | null>(null);

  const id = () => idOf(draft);
  const field = () => keyField(held(), id());
  const reference = () => secretFor(held(), id()) ?? referenceOf(id());
  // The probe's answer for the endpoint this form is about. A probe now
  // answers with a reading whether or not it could read a model list, so
  // this is also where the reachability report comes from.
  const answered = createMemo<Probed | null>(() => {
    const found = ui.conn.belief.probed;
    return found !== null && found.name === id() ? found : null;
  });
  const facts = createMemo(() => answered()?.facts ?? []);
  const dialect = (): DialectKind | null => dialectOf(draft.wireApi);
  const complete = () => ID_SHAPE.test(id()) && hostOf(draft.baseUrl) !== null && dialect() !== null;

  // A refusal that arrives while this form is waiting for one is this
  // form's to show, beside the field it is about; the corner is for
  // refusals no open page is responsible for.
  createEffect(() => {
    const refused = ui.conn.belief.refusal;
    if (refused === null || !looking()) return;
    setReport(refused);
    setLooking(false);
    ui.conn.dismissRefusal();
  });
  // An endpoint that answered an empty list answered: the wait ends on
  // the answer rather than on its length.
  createEffect(() => {
    if (answered() !== null) setLooking(false);
  });
  // The city writes down the base URL it will call. When that is not
  // the text in the box, the box takes the city's spelling and the line
  // under it names what was replaced, so the person leaves knowing the
  // URL that took effect. **The rule that produced it is the city's**
  // (`gateway::normalise_entered`); this compares two strings and
  // nothing more, which is why the form cannot drift from the call.
  //
  // Rewriting the box does not move the answer out from under it:
  // normalisation settles a scheme and a path and never the host, and
  // the derived id this probe was filed under is a function of the
  // host alone.
  createEffect(() => {
    const found = answered();
    if (found === null) return;
    const moved = normalisedFrom(found, draft.baseUrl);
    if (moved === null) return;
    setRewritten(moved.entered);
    setDraft("baseUrl", moved.recorded);
  });

  // Every edit retires the last report: what it said was about the
  // fields as they were.
  const edit = (part: "baseUrl" | "key", value: string) => {
    setDraft(part, value);
    setReport(null);
    setNote(null);
    if (part === "baseUrl") setRewritten(null);
  };

  // The key is enrolled on the action that needs it, under the id the
  // form says now. A blank box keeps whatever is already filed under
  // that id; a box with something in it replaces it.
  const withKey = (then: (e: Endpoint) => void) => {
    const typed = draft.key.trim();
    const keep = secretFor(held(), id());
    if (typed === "") {
      then(endpointOf(draft, keep));
      return;
    }
    setBusy(true);
    const { realm, name } = referenceFor(id());
    const settle = (outcome: Enrolment) => {
      setBusy(false);
      if (outcome.kind !== "stored") {
        setNote(outcome.reason);
        return;
      }
      setHeld({ provider: id(), reference: outcome.reference });
      setDraft("key", "");
      then(endpointOf(draft, outcome.reference));
    };
    void enrol({ origin: ui.origin, realm, name, value: typed, lang: ui.prefs.held().lang }).then(
      settle,
    );
  };

  return (
    <form
      class="flex flex-col gap-base"
      onSubmit={(event) => {
        event.preventDefault();
      }}
    >
      <div class="flex flex-col gap-tight">
        <Field
          label={say("setup_base_url")}
          kind="url"
          pattern={BASE_URL.source}
          mono
          value={draft.baseUrl}
          // wording-ok: an address, which no language translates
          placeholder="https://api.openai.com/v1"
          onInput={(value) => { edit("baseUrl", value); }}
        />
        <Show when={rewritten()}>
          {(from) => (
            <span class="text-note text-text-faint">
              {say("setup_base_url_normalised", { entered: from() })}
            </span>
          )}
        </Show>
      </div>
      <div class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_wire_api")}
        <Segmented
          label={say("setup_wire_api")}
          options={wireChoices()}
          held={draft.wireApi}
          onPick={(api) => { setDraft("wireApi", api); }}
        />
      </div>
      <div class="flex flex-col gap-tight text-note text-text-quiet">
        <Field
          label={say("setup_key")}
          kind="password"
          mono
          value={draft.key}
          onInput={(value) => { edit("key", value); }}
        />
        <Show when={field().kind === "stored"}>
          <span class="text-text-faint">{say("setup_key_stored")}</span>
          <span class="font-mono text-text-disabled">{reference()}</span>
          <span class="flex gap-snug">
            <button
              type="button"
              class="rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
              onClick={() => {
                setHeld(null);
              }}
            >
              {say("setup_key_replace")}
            </button>
            <button
              type="button"
              class="rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
              onClick={() => {
                setHeld(null);
                setDraft("key", "");
              }}
            >
              {say("setup_key_clear")}
            </button>
          </span>
          <span class="text-text-faint">{say("setup_key_vault_note")}</span>
        </Show>
      </div>

      <div class="flex flex-col gap-snug">
        <button
          type="button"
          class="self-start rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2"
          aria-expanded={advanced()}
          onClick={() => setAdvanced(!advanced())}
        >
          {say("setup_advanced")}
        </button>
        <Show when={advanced()}>
          <AdvancedFields
            draft={draft}
            setDraft={setDraft}
            onRenamed={() => {
              setReport(null);
              setNote(null);
            }}
          />
        </Show>
      </div>

      <Show when={note()}>{(text) => <p class="text-note text-alert">{text()}</p>}</Show>
      <Show when={report()}>
        {(error) => <Refusal error={error()} host={hostOf(draft.baseUrl) ?? draft.baseUrl} />}
      </Show>
      <Show when={answered()}>{(found) => <Reachability probed={found()} />}</Show>
      <Show when={looking()}>
        <p class="text-note text-text-quiet">{say("setup_probe_busy", { host: hostOf(draft.baseUrl) ?? "" })}</p>
      </Show>

      <div class="flex flex-wrap items-center gap-snug">
        <button
          type="button"
          class="rounded-control bg-g2 px-base py-snug text-label hover:bg-g3 disabled:text-text-disabled"
          disabled={!complete() || busy()}
          onClick={() => {
            setReport(null);
            withKey((e) => {
              if (command(probeEndpoint(e))) setLooking(true);
            });
          }}
        >
          {say("setup_look")}
        </button>
        <button
          type="button"
          class="rounded-control bg-accent px-base py-snug text-label text-g0 hover:bg-accent-hover disabled:bg-g3 disabled:text-text-disabled"
          disabled={!complete() || busy()}
          onClick={() => {
            setReport(null);
            withKey((e) => {
              const chosen = rows();
              if (!command(attachEndpoint(e, chosen.map((row) => row.id)))) return;
              for (const row of chosen) {
                if (row.tag !== null) command(selectModel(e.id, row.id, row.tag, row.ceilings));
              }
              // The form starts over for the next provider. Only a
              // registration that went through clears it: a refusal
              // keeps every field, which is the one thing a person
              // filling in a key cannot be asked to do twice.
              setDraft(freshDraft());
              setHeld(null);
              setRows([]);
              setRewritten(null);
              props.onAttached?.();
            });
          }}
        >
          {say("setup_attach_btn")}
        </button>
      </div>

      <ModelTable served={facts()} onChosen={setRows} />

      <Previews draft={draft} reference={reference()} model={rows()[0]?.id ?? facts()[0]?.id ?? "<model>"} />
    </form>
  );
}
