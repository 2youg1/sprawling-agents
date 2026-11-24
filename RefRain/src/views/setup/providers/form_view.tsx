// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, Show, untrack } from "solid-js";

import { readAnswer } from "../../../core/answered";
import { QUERIES } from "../../../core/asking";
import {
  attachEndpoint,
  probeEndpoint,
  selectModel,
  type Endpoint,
  type WireApi,
} from "../../../core/commands";
import { enrol, keyField, referenceFor, secretFor, type StoredKey } from "../../../core/enrol";
import { fill, say, type Key } from "../../../core/lang";
import { normalisedFrom, refusalOf, type Sent } from "../../../core/probed";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import type { AxError, EndpointSummary, IdemKey } from "../../../wire";
import Button from "../../parts/button.solid";
import Combobox from "../../parts/combobox_view";
import Field from "../../parts/field.solid";
import Segmented from "../../parts/segmented.solid";
import { RECEIPT_MS } from "../../settings/saving";
import type { TableAbout } from "../model_rows";
import ModelTable from "../model_table.solid";
import type { ModelRow } from "../models";
import Advanced from "./advanced_view";
import {
  BASE_URL,
  ID_SHAPE,
  endpointOf,
  freshDraft,
  hostOf,
  idOf,
  referenceOf,
  wireChoices,
  type Draft,
} from "./draft";
import { facesOf, followed, hostChoices, picked, rowFor } from "./known";
import Previews from "./preview_view";
import Reach from "./reach_view";

export interface AttachFormProps {
  readonly onAttached?: (() => void) | undefined;
}

type Act = "probe" | "attach";

interface Probe extends Sent {
  readonly idem: IdemKey;
}

interface Attaching {
  readonly idem: IdemKey;
  readonly endpoint: Endpoint;
  readonly draft: Draft;
  readonly rows: readonly ModelRow[];
  readonly before: string | undefined;
}

function fingerprint(endpoint: EndpointSummary): string {
  return JSON.stringify({
    address: endpoint.base_url,
    dialect: endpoint.dialect,
    credential: endpoint.has_credential,
    tuning: endpoint.tuning,
    models: endpoint.models.map((model) => model.id).sort(),
  });
}

function location(url: string): string {
  const stripped = url.trim().replace(/^https?:\/\//i, "");
  return (stripped.split("/")[0] ?? "").toLowerCase();
}

export default function AttachForm(props: AttachFormProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const receipts = storeValue(u.conn.receipts);
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const knownAnswer = storeValue(u.conn.asking.ask(QUERIES.knownHosts));
  const [draft, replaceDraft] = createSignal(freshDraft());
  const [held, setHeld] = createSignal<StoredKey | null>(null);
  const [note, setNote] = createSignal<string>();
  const [report, setReport] = createSignal<AxError>();
  const [enrolling, setEnrolling] = createSignal<Act>();
  const [looking, setLooking] = createSignal(false);
  const [attaching, setAttaching] = createSignal<Attaching>();
  const [unknown, setUnknown] = createSignal<Act>();
  const [lastProbe, setLastProbe] = createSignal<Probe>();
  const [chosen, setChosen] = createSignal<readonly ModelRow[]>([]);
  const [modelTable, setModelTable] = createSignal(0);
  const word = (key: Key) => say(lang(), key);
  const id = () => idOf(draft());
  const host = () => hostOf(draft().baseUrl) ?? draft().baseUrl;
  const field = () => keyField(held(), id());
  const reference = () => {
    const key = field();
    return key.kind === "stored" ? key.reference : referenceOf(id());
  };
  const complete = () => ID_SHAPE.test(id()) && hostOf(draft().baseUrl) !== null;
  const answer = createMemo(() => {
    const received = endpoints();
    return received !== undefined && "endpoints" in received ? received.endpoints : undefined;
  });
  const keyed = () =>
    answer()?.endpoints.some((endpoint) => endpoint.name === id() && endpoint.has_credential) ===
    true;
  const probed = createMemo(() => {
    const received = belief().probed;
    const current = lastProbe();
    return current !== undefined &&
      current.id === id() &&
      current.baseUrl === draft().baseUrl &&
      current.wireApi === draft().wireApi &&
      received !== null &&
      received !== current.before &&
      received.name === current.id
      ? received
      : null;
  });
  const facts = () => probed()?.facts ?? [];
  const called = () => normalisedFrom(probed(), lastProbe() ?? null, draft());
  const about = createMemo<TableAbout | null>(() => {
    if (!complete()) return null;
    const endpoint = endpointOf(draft(), null);
    return { endpoint: endpoint.id, baseUrl: endpoint.baseUrl, face: endpoint.dialect };
  });
  const known = createMemo(() =>
    readAnswer(knownAnswer(), (answer) =>
      "known_hosts" in answer ? answer.known_hosts.hosts : undefined,
    ),
  );
  const hosts = createMemo(() => {
    const answer = known();
    return answer.kind === "held" ? answer.value : [];
  });
  const row = () => rowFor(hosts(), draft().baseUrl);
  const faces = createMemo(() => {
    const picked = row();
    const allowed = picked === null ? null : facesOf(picked);
    return wireChoices().map((choice) =>
      allowed !== null && !allowed.includes(choice.value)
        ? { ...choice, why: fill(word("setup_face_off"), { faces: allowed.join(" / ") }) }
        : choice,
    );
  });
  const reachableError = () => {
    const actual = report();
    return actual !== undefined && refusalOf(actual.code) === "reach" ? actual : null;
  };
  const formError = () => {
    const actual = report();
    return actual !== undefined && refusalOf(actual.code) === "form" ? actual : null;
  };
  const check = (): void => u.conn.asking.refresh(QUERIES.endpoints);
  const provider = (name: string): EndpointSummary | undefined =>
    answer()?.endpoints.find((item) => item.name === name);

  createEffect(
    () => ({ active: looking(), result: probed(), probe: lastProbe(), receipts: receipts() }),
    ({ active, result, probe, receipts }) => {
      if (!active || probe === undefined) return;
      const outcome = receipts.get(probe.idem);
      if (outcome?.kind === "refused") {
        setReport(outcome.error);
        setLooking(false);
        setUnknown(undefined);
      } else if (result !== null) {
        setLooking(false);
        setUnknown(undefined);
      }
    },
  );
  createEffect(
    () => {
      const work = attaching();
      return {
        work,
        actual: work === undefined ? undefined : provider(work.endpoint.id),
        receipts: receipts(),
      };
    },
    ({ work, actual, receipts }) => {
      if (work === undefined) return;
      const outcome = receipts.get(work.idem);
      if (outcome?.kind === "refused") {
        setReport(outcome.error);
        setAttaching(undefined);
        setUnknown(undefined);
        return;
      }
      if (
        actual === undefined ||
        actual.dialect !== work.endpoint.dialect ||
        location(actual.base_url) !== location(work.endpoint.baseUrl) ||
        !work.rows.every((row) => actual.models.some((model) => model.id === row.id)) ||
        (outcome?.kind !== "accepted" && fingerprint(actual) === work.before)
      )
        return;
      for (const row of work.rows) {
        if (row.tag !== null) u.send(selectModel(work.endpoint.id, row.id, row.tag, row.stated));
      }
      if (untrack(draft) === work.draft) {
        replaceDraft(freshDraft());
        setHeld(null);
        setChosen([]);
        setLastProbe(undefined);
        setModelTable((revision) => revision + 1);
      }
      setAttaching(undefined);
      setUnknown(undefined);
      props.onAttached?.();
    },
  );
  createEffect(
    () => ({ probe: looking(), attach: attaching() }),
    ({ probe, attach }) => {
      if (!probe && attach === undefined) return;
      const poll = attach === undefined ? undefined : window.setInterval(check, 1_000);
      const timer = window.setTimeout(() => {
        setUnknown(attach === undefined ? "probe" : "attach");
        window.clearInterval(poll);
      }, RECEIPT_MS);
      return () => {
        window.clearTimeout(timer);
        window.clearInterval(poll);
      };
    },
  );

  function retire(): void {
    setReport(undefined);
    setNote(undefined);
  }

  function setDraft<K extends keyof Draft>(part: K, value: Draft[K]): void {
    replaceDraft((before) => ({ ...before, [part]: value }));
    retire();
  }

  function chooseHost(host: string): void {
    const item = hosts().find((entry) => entry.host === host);
    const next = item === undefined ? null : picked(item, draft().wireApi);
    if (next === null) return;
    replaceDraft((current) => ({ ...current, ...next }));
    retire();
  }

  function chooseApi(wireApi: WireApi): void {
    const known = row();
    const switched = known === null ? null : followed(known, draft().baseUrl, wireApi);
    replaceDraft((current) => ({ ...current, wireApi, baseUrl: switched ?? current.baseUrl }));
    retire();
  }

  function submit(mode: Act, captured: Draft, endpoint: Endpoint, rows: readonly ModelRow[]): void {
    if (mode === "probe") {
      const command = probeEndpoint(endpoint);
      if (!("probe_endpoint" in command) || !u.send(command)) {
        setNote(word("setup_account_not_sent"));
        return;
      }
      setLastProbe({
        idem: command.probe_endpoint.idem,
        id: endpoint.id,
        baseUrl: captured.baseUrl,
        wireApi: captured.wireApi,
        before: belief().probed,
      });
      setLooking(true);
      return;
    }
    const listed = (id: string) => facts().some((row) => row.id === id);
    const admit = rows.filter((row) => listed(row.id)).map((row) => row.id);
    const named = rows.filter((row) => !listed(row.id)).map((row) => row.id);
    const command = attachEndpoint(endpoint, admit, named);
    if (!("attach_endpoint" in command) || !u.send(command)) {
      setNote(word("setup_account_not_sent"));
      return;
    }
    const before = provider(endpoint.id);
    setAttaching({
      idem: command.attach_endpoint.idem,
      endpoint,
      draft: captured,
      rows,
      before: before === undefined ? undefined : fingerprint(before),
    });
    check();
  }

  function start(mode: Act): void {
    if (!complete() || enrolling() !== undefined || attaching() !== undefined) return;
    if (looking() && unknown() !== "probe") return;
    retire();
    setLooking(false);
    setUnknown(undefined);
    const captured = draft();
    const rows = chosen();
    const provider = idOf(captured);
    const value = captured.key.trim();
    if (value === "") {
      submit(mode, captured, endpointOf(captured, secretFor(held(), provider)), rows);
      return;
    }
    setEnrolling(mode);
    const { realm, name } = referenceFor(provider);
    void enrol({
      origin: u.origin,
      token: u.credential(),
      realm,
      name,
      value,
      lang: lang(),
    }).then((outcome) => {
      setEnrolling(undefined);
      if (outcome.kind === "refused") {
        setNote(outcome.reason);
        return;
      }
      setHeld({ provider, reference: outcome.reference });
      const cleared = { ...captured, key: "" };
      if (draft() === captured) replaceDraft(cleared);
      submit(mode, cleared, endpointOf(captured, outcome.reference), rows);
    });
  }

  const busy = () =>
    enrolling() !== undefined ||
    (looking() && unknown() === undefined) ||
    (attaching() !== undefined && unknown() === undefined);
  const blocked = () => (complete() ? {} : { why: word("setup_form_incomplete") });
  return (
    <form class="flex flex-col gap-wide" onSubmit={(event) => event.preventDefault()}>
      <fieldset class="flex min-w-0 flex-col gap-base border-0 p-0">
        <legend class="px-0 text-note font-label text-text-quiet">
          {word("setup_form_provider")}
        </legend>
        <Combobox
          label={word("setup_known_host")}
          placeholder={word("setup_known_host_pick")}
          empty={word("part_no_match")}
          choices={hostChoices(hosts())}
          value={row()?.host ?? null}
          onPick={chooseHost}
        />
        <div class="flex flex-col gap-tight">
          <Field
            placeholder={word("setup_base_url_example")}
            label={word("setup_base_url")}
            pattern={BASE_URL.source}
            mono
            value={draft().baseUrl}
            onInput={(value) => setDraft("baseUrl", value)}
          />
          <Show when={called()}>
            {(recorded) => (
              <span class="text-note text-text-faint">
                {fill(word("setup_base_url_normalised"), { recorded: recorded() })}
              </span>
            )}
          </Show>
        </div>
        <div class="flex flex-col gap-tight text-note text-text-quiet">
          {word("setup_wire_api")}
          <Segmented
            label={word("setup_wire_api")}
            options={faces()}
            held={draft().wireApi}
            onPick={chooseApi}
          />
        </div>
        <Advanced draft={draft()} setDraft={setDraft} onRenamed={retire} />
      </fieldset>

      <fieldset class="flex min-w-0 flex-col gap-base border-0 p-0">
        <legend class="px-0 text-note font-label text-text-quiet">
          {word("setup_form_credentials")}
        </legend>
        <div class="flex flex-col gap-tight text-note text-text-quiet">
          <Field
            label={word("setup_key")}
            kind="password"
            mono
            value={draft().key}
            {...(keyed() ? { placeholder: word("setup_key_kept") } : {})}
            onInput={(value) => setDraft("key", value)}
          />
          <Show when={field().kind === "stored"}>
            <span class="text-text-faint">{word("setup_key_stored")}</span>
            <span class="font-mono text-text-faint">{reference()}</span>
            <span class="flex gap-snug">
              <Button
                label={word("setup_key_replace")}
                tone="quiet"
                glyph="key"
                onPress={() => setHeld(null)}
              />
              <Button
                label={word("setup_key_clear")}
                tone="quiet"
                glyph="cross"
                onPress={() => {
                  setHeld(null);
                  setDraft("key", "");
                }}
              />
            </span>
            <span class="text-text-faint">{word("setup_key_vault_note")}</span>
          </Show>
        </div>
      </fieldset>

      <fieldset class="flex min-w-0 flex-col gap-base border-0 p-0">
        <legend class="px-0 text-note font-label text-text-quiet">
          {word("setup_form_models")}
        </legend>
        <div class="flex flex-wrap items-center gap-snug">
          <Button
            label={word("setup_look")}
            tone="secondary"
            glyph="search"
            loading={enrolling() === "probe" || (looking() && unknown() === undefined)}
            {...(attaching() === undefined && enrolling() !== "attach"
              ? blocked()
              : { why: word("saving_sent") })}
            onPress={() => start("probe")}
          />
          <Show
            when={unknown() === "attach"}
            fallback={
              <Button
                label={word("setup_attach_btn")}
                tone="primary"
                glyph="door"
                loading={attaching() !== undefined || enrolling() === "attach"}
                {...(busy() && attaching() === undefined && enrolling() !== "attach"
                  ? { why: word("saving_sent") }
                  : blocked())}
                onPress={() => start("attach")}
              />
            }
          >
            <Button label={word("saving_check")} tone="primary" result="unknown" onPress={check} />
          </Show>
        </div>
        <Show when={note()}>
          <p role="alert" class="text-note text-alert">
            {note()}
          </p>
        </Show>
        <Reach probed={probed()} error={reachableError()} host={host()} />
        <Show when={looking() || attaching() !== undefined}>
          <p role="status" class="text-note text-text-quiet">
            {unknown() !== undefined
              ? word("saving_unverified")
              : attaching() !== undefined
                ? word("saving_sent")
                : fill(word("setup_probe_busy"), { host: host() })}
          </p>
        </Show>
        <For each={[modelTable()]} keyed={true}>
          {() => (
            <ModelTable
              served={facts()}
              about={about()}
              onChosen={(rows) => {
                setChosen(rows);
              }}
            />
          )}
        </For>
        <Reach probed={null} error={formError()} host={host()} />
        <Previews
          draft={draft()}
          reference={reference()}
          model={chosen()[0]?.id ?? facts()[0]?.id ?? "<model>"}
        />
      </fieldset>
    </form>
  );
}
