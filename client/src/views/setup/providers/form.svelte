<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Turning a URL and a key into models a run can be given.
  //
  // **Three boxes and two controls are all the form asks for.** A URL,
  // which face the endpoint answers in, a key, and then look or attach.
  // Above the URL stands the city's list of the vendors it knows by
  // host (`./known`): picking one fills the URL and the face, so a
  // person copies no address out of a vendor's documentation;
  // the id and the display name are derived from the URL and folded
  // into `advanced`, because they are the city's bookkeeping rather
  // than anything the person came holding. What each field means is
  // `./draft`; this file is what the form does with them.
  //
  // The key goes to the vault by its own route and comes back as a
  // reference. **An empty key box carries no reference at all**: the
  // city reads that as "keep what is archived under this id", so this
  // form never decides "keep" out of what it happens to remember
  // (sprawling-SPEC 8-81). The reference is held beside the id it was
  // filed under, because a reference held alone outlives the name that
  // makes it true.

  export interface AttachFormProps {
    // A registration went through and the form is empty again.
    readonly onAttached?: () => void;
  }
</script>

<script lang="ts">
  import { get } from "svelte/store";

  import { QUERIES } from "../../../core/asking";
  import { attachEndpoint, probeEndpoint, selectModel } from "../../../core/commands";
  import type { Endpoint, WireApi } from "../../../core/commands";
  import { enrol, keyField, referenceFor, secretFor } from "../../../core/enrol";
  import type { Enrolment, StoredKey } from "../../../core/enrol";
  import { readAnswer } from "../../../core/answered";
  import { fill, say } from "../../../core/lang";
  import { normalisedFrom } from "../../../core/probed";
  import type { AxError } from "../../../wire";
  import { ui } from "../../../ui";
  import Button from "../../parts/button.svelte";
  import Combobox from "../../parts/combobox.svelte";
  import Field from "../../parts/field.svelte";
  import Segmented from "../../parts/segmented.svelte";
  import type { Choice } from "../../parts/segmented";
  import { ModelTable, type ModelRow } from "../models";
  import Advanced from "./advanced.svelte";
  import { BASE_URL, ID_SHAPE, endpointOf, freshDraft, hostOf, idOf, referenceOf, wireChoices } from "./draft";
  import type { Draft } from "./draft";
  import { facesOf, followed, hostChoices, picked, rowFor } from "./known";
  import Previews from "./preview.svelte";
  import Reach from "./reach.svelte";

  const { onAttached }: AttachFormProps = $props();
  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  let draft = $state<Draft>(freshDraft());
  // The reference the vault answered with, and the id it was filed
  // under. Never one without the other.
  let held = $state<StoredKey | null>(null);
  let note = $state<string | null>(null);
  let report = $state<AxError | null>(null);
  let looking = $state(false);
  // The attach this form sent and is waiting on, with the endpoints
  // answer that stood when it left: the form starts over only once a
  // newer answer lists the endpoint, and a refusal keeps every field.
  let attaching = $state.raw<{ readonly id: string; readonly before: unknown } | null>(null);
  let busy = $state(false);
  let chosen = $state<readonly ModelRow[]>([]);
  // What the URL box held before the city answered with another
  // spelling of the same endpoint.
  let rewritten = $state<string | null>(null);

  const id = $derived(idOf(draft));
  const field = $derived(keyField(held, id));
  const reference = $derived(field.kind === "stored" ? field.reference : referenceOf(id));
  // Whether the city keeps a key for the id this form is about. The
  // reference itself never travels here - `has_credential` is the
  // whole answer a page needs - and the city leaves what it keeps
  // alone when this form sends no key. So an empty box on a keyed
  // endpoint changes nothing, and the placeholder is what says so.
  const attached = u.conn.asking.ask(QUERIES.endpoints);
  const keyed = $derived.by(() => {
    const now = $attached;
    if (now === undefined || !("endpoints" in now)) return false;
    return now.endpoints.endpoints.some((each) => each.name === id && each.has_credential);
  });
  // The probe's answer for the endpoint this form is about. A probe
  // answers with a reading whether or not it could read a model list,
  // so this is also where the reachability report comes from.
  const answered = $derived.by(() => {
    const found = $belief.probed;
    return found !== null && found.name === id ? found : null;
  });
  const facts = $derived(answered?.facts ?? []);
  const complete = $derived(ID_SHAPE.test(id) && hostOf(draft.baseUrl) !== null);

  // The vendors the city knows by host. The answer is the city's own
  // table, so the picker offers no address and no face the city would
  // not store.
  const knownAnswer = u.conn.asking.ask(QUERIES.knownHosts);
  const known = $derived(
    readAnswer($knownAnswer, (held) => ("known_hosts" in held ? held.known_hosts.hosts : undefined)),
  );
  const hosts = $derived(known.kind === "held" ? known.value : []);
  const row = $derived(rowFor(hosts, draft.baseUrl));

  // The face control, with the cells a known host does not answer in
  // refused under the pointer and saying which shapes it does speak. A
  // host the city does not know keeps every cell open: the form makes
  // the widest judgement and never a narrower one (client-SPEC 4-25).
  const faces = $derived.by((): readonly Choice<WireApi>[] => {
    const offered = row === null ? null : facesOf(row);
    return wireChoices().map((choice) =>
      offered !== null && !offered.includes(choice.value)
        ? { ...choice, why: fill(say($lang, "setup_face_off"), { faces: offered.join(" / ") }) }
        : choice,
    );
  });

  // A refusal that arrives while this form is waiting for one is this
  // form's to show, beside the field it is about; the corner is for
  // refusals no open page is responsible for.
  $effect(() => {
    const refused = $belief.refusal;
    if (refused === null || (!looking && attaching === null)) return;
    report = refused;
    looking = false;
    attaching = null;
    u.conn.dismissRefusal();
  });
  // The city took the endpoint: the answer after the one that stood when
  // the attach left lists it. The form starts over for the next provider.
  $effect(() => {
    const now = $attached;
    if (attaching === null || now === attaching.before || now === undefined || !("endpoints" in now)) return;
    const taken = attaching.id;
    if (!now.endpoints.endpoints.some((each) => each.name === taken)) return;
    attaching = null;
    draft = freshDraft();
    held = null;
    chosen = [];
    rewritten = null;
    onAttached?.();
  });
  // An endpoint that answered an empty list answered: the wait ends on
  // the answer rather than on its length.
  $effect(() => {
    if (answered !== null) looking = false;
  });
  // The city writes down the base URL it will call. When that is not
  // the text in the box, the box takes the city's spelling and the
  // line under it names what was replaced, so the person leaves
  // knowing the URL that took effect. **The rule that produced it is
  // the city's** (`gateway::normalise_entered`); this compares two
  // strings and nothing more, which is why the form cannot drift from
  // the call. Rewriting the box does not move the answer out from
  // under it: normalisation settles a scheme and a path and never the
  // host, and the id this probe was filed under is a function of the
  // host alone.
  $effect(() => {
    const found = answered;
    if (found === null) return;
    const moved = normalisedFrom(found, draft.baseUrl);
    if (moved === null) return;
    rewritten = moved.entered;
    draft.baseUrl = moved.recorded;
  });

  // Every edit retires the last report: what it said was about the
  // fields as they were.
  function edit(part: "baseUrl" | "key", value: string): void {
    draft[part] = value;
    report = null;
    note = null;
    if (part === "baseUrl") rewritten = null;
  }

  function retire(): void {
    report = null;
    note = null;
  }

  // What the fold below edits through: the draft is this form's own
  // state, so the only hand on it is here (the Solid original passed
  // its store setter for the same reason).
  function setDraft<K extends keyof Draft>(part: K, value: Draft[K]): void {
    draft[part] = value;
  }

  // The key is enrolled on the action that needs it, under the id the
  // form says now. A blank box carries the reference this form enrolled
  // for that id, if any: listing the models clears the box, and the
  // attach that follows must not arrive without the key it just filed.
  // A box with something in it replaces what is filed.
  function withKey(then: (e: Endpoint) => void): void {
    const typed = draft.key.trim();
    if (typed === "") {
      then(endpointOf(draft, secretFor(held, id)));
      return;
    }
    busy = true;
    const { realm, name } = referenceFor(id);
    const settle = (outcome: Enrolment): void => {
      busy = false;
      if (outcome.kind !== "stored") {
        note = outcome.reason;
        return;
      }
      held = { provider: id, reference: outcome.reference };
      draft.key = "";
      then(endpointOf(draft, outcome.reference));
    };
    void enrol({
      origin: u.origin,
      token: u.pairing,
      realm,
      name,
      value: typed,
      lang: get(lang),
    }).then(settle);
  }

  function probeNow(): void {
    report = null;
    withKey((e) => {
      if (u.send(probeEndpoint(e))) looking = true;
    });
  }

  function attachNow(): void {
    report = null;
    withKey((e) => {
      const rows = chosen;
      const before = get(attached);
      if (!u.send(attachEndpoint(e, rows.map((row) => row.id)))) return;
      for (const row of rows) {
        if (row.tag !== null) u.send(selectModel(e.id, row.id, row.tag, row.stated));
      }
      // Only a registration that went through clears the form: a
      // refusal keeps every field, which is the one thing a person
      // filling in a key cannot be asked to do twice.
      attaching = { id: e.id, before };
    });
  }
</script>

<form
  class="flex flex-col gap-base"
  onsubmit={(event) => {
    event.preventDefault();
  }}
>
  <Combobox
    label={say($lang, "setup_known_host")}
    placeholder={say($lang, "setup_known_host_pick")}
    empty={say($lang, "part_no_match")}
    choices={hostChoices(hosts)}
    value={row?.host ?? null}
    onPick={(host: string) => {
      const chosenRow = hosts.find((each) => each.host === host);
      const filled = chosenRow === undefined ? null : picked(chosenRow, draft.wireApi);
      if (filled === null) return;
      draft.wireApi = filled.wireApi;
      edit("baseUrl", filled.baseUrl);
    }}
  />

  <div class="flex flex-col gap-tight">
    <!-- wording-ok: the placeholder is the provider's documented base URL - an address, which no language translates. -->
    <Field placeholder="https://api.openai.com/v1"
      label={say($lang, "setup_base_url")}
      kind="url"
      pattern={BASE_URL.source}
      mono
      value={draft.baseUrl}
      onInput={(value: string) => {
        edit("baseUrl", value);
      }}
    />
    {#if rewritten !== null}
      <span class="text-note text-text-faint">
        {fill(say($lang, "setup_base_url_normalised"), { entered: rewritten })}
      </span>
    {/if}
  </div>

  <div class="flex flex-col gap-tight text-note text-text-quiet">
    {say($lang, "setup_wire_api")}
    <Segmented
      label={say($lang, "setup_wire_api")}
      options={faces}
      held={draft.wireApi}
      onPick={(api: WireApi) => {
        draft.wireApi = api;
        const moved = row === null ? null : followed(row, draft.baseUrl, api);
        if (moved !== null) edit("baseUrl", moved);
      }}
    />
  </div>

  <div class="flex flex-col gap-tight text-note text-text-quiet">
    <Field
      label={say($lang, "setup_key")}
      kind="password"
      mono
      value={draft.key}
      {...(keyed ? { placeholder: say($lang, "setup_key_kept") } : {})}
      onInput={(value: string) => {
        edit("key", value);
      }}
    />
    {#if field.kind === "stored"}
      <span class="text-text-faint">{say($lang, "setup_key_stored")}</span>
      <span class="font-mono text-text-faint">{reference}</span>
      <span class="flex gap-snug">
        <Button
          label={say($lang, "setup_key_replace")}
          tone="quiet"
          onPress={() => {
            held = null;
          }}
        />
        <Button
          label={say($lang, "setup_key_clear")}
          tone="quiet"
          onPress={() => {
            held = null;
            draft.key = "";
          }}
        />
      </span>
      <span class="text-text-faint">{say($lang, "setup_key_vault_note")}</span>
    {/if}
  </div>

  <div class="flex flex-wrap items-center gap-snug">
    <Button
      label={say($lang, "setup_look")}
      tone="secondary"
      loading={busy}
      {...(complete ? {} : { why: say($lang, "setup_form_incomplete") })}
      onPress={probeNow}
    />
    <Button
      label={say($lang, "setup_attach_btn")}
      tone="primary"
      loading={busy}
      {...(complete ? {} : { why: say($lang, "setup_form_incomplete") })}
      onPress={attachNow}
    />
  </div>

  <!-- What came back stands under the controls that asked for it,
  then the boxes that stay folded away, then the two readings of the
  draft at the foot. -->
  {#if note !== null}
    <p class="text-note text-alert">{note}</p>
  {/if}
  <Reach probed={answered} error={report} host={hostOf(draft.baseUrl) ?? draft.baseUrl} />
  {#if looking}
    <p class="text-note text-text-quiet">
      {fill(say($lang, "setup_probe_busy"), { host: hostOf(draft.baseUrl) ?? "" })}
    </p>
  {/if}

  <Advanced {draft} {setDraft} onRenamed={retire} />

  <ModelTable
    served={facts}
    onChosen={(rows: readonly ModelRow[]) => {
      chosen = rows;
    }}
  />

  <Previews draft={draft} reference={reference} model={chosen[0]?.id ?? facts[0]?.id ?? "<model>"} />
</form>
