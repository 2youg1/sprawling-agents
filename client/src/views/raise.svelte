<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The page a building is raised on, filled in once and written once
  // (client/Spec.lean 4-50b): the address, the template, and the
  // `RULES.toml` that template would write, read from the city and
  // editable before anything lands. An address naming a folder already
  // in the city adopts it; the city decides that by looking, and the page
  // says which it will be before the press. The city reads the rules
  // before anything is laid out, so a refused text leaves nothing behind
  // and stays in the box to be corrected.
  import { untrack } from "svelte";

  import { QUERIES } from "../core/asking";
  import { createBuilding } from "../core/commands";
  import type { Template } from "../core/commands";
  import { fill, say } from "../core/lang";
  import { ui } from "../ui";
  import { TemplateName } from "../wire";
  import type { Address, AxError } from "../wire";
  import Button from "./parts/button.svelte";
  import Field from "./parts/field.svelte";
  import { noticeTitle } from "./parts/notice_title";
  import Page from "./parts/page.svelte";
  import Segmented from "./parts/segmented.svelte";
  import { textBoxWire } from "./building/text_box";
  import TextBox from "./building/text_box.look.svelte";
  import { fillable, OFFERED, raisingOf } from "./raise";

  interface Props {
    // The address a link filled in: a folder waiting to be adopted.
    readonly address?: Address | undefined;
    // Whether this is the page or a fixture inside one.
    readonly rank?: "page" | "section" | undefined;
  }

  const { address, rank = "page" }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const city = u.conn.asking.ask(QUERIES.city);

  // svelte-ignore state_referenced_locally
  let typed = $state<string>(address ?? "");
  // The address box and the folders offered under it. A pressed folder
  // leaves the list it was pressed in, so the caret goes to the box the
  // folder was written into rather than back to the top of the page.
  let addressGroup = $state<HTMLDivElement>();
  let template = $state<Exclude<Template, "hall">>("minimal");
  let rules = $state("");
  // Whether the person changed the rules: until then the box follows
  // the template and the address. The rules belong to that pair, so a
  // new address or template starts the box over from its bytes (4-50b):
  // edited ordinary rules sent under the confidential template would
  // raise the ordinary building the person had just turned away from.
  let edited = $state(false);
  // The raise in flight, and the refusal the city held when it left, so
  // only a newer one is read as this raise's answer.
  let sent = $state.raw<{ readonly addr: Address; readonly before: AxError | null } | null>(null);
  let refusal = $state.raw<AxError | null>(null);

  const buildings = $derived($city !== undefined && "city" in $city ? $city.city.buildings : []);
  const raising = $derived(raisingOf(typed, buildings));
  const target = $derived(raising.kind === "build" || raising.kind === "adopt" ? raising.addr : null);

  // The rules the template would write here, asked of the city, and
  // whether the city has answered at all: an address it will not lay a
  // template at is answered without rules, and the press then brings
  // back its reason.
  let offered = $state<string | null>(null);
  let answered = $state(false);
  $effect(() => {
    const at = target;
    const picked = template;
    offered = null;
    answered = false;
    edited = false;
    rules = "";
    if (at === null) return;
    return u.conn.asking.ask({ template: { template: TemplateName.make(picked), addr: at } }).subscribe((answer) => {
      offered = answer !== undefined && "template" in answer ? answer.template.rules : null;
      answered = answer !== undefined;
    });
  });
  // Whether the press sends rules the person has seen: their own, or the
  // template's once the city has said what those are.
  const ready = $derived(target !== null && (edited || answered));
  $effect(() => {
    const text = offered;
    if (text !== null && !untrack(() => edited)) rules = text;
  });

  // The city's answer to the raise: the building listed with its rules,
  // or a refusal newer than the one held when it left.
  $effect(() => {
    const held = sent;
    if (held === null) return;
    if (buildings.some((each) => each.addr === held.addr && !each.adoptable)) {
      sent = null;
      u.go({ kind: "building", address: held.addr });
      return;
    }
    const error = $belief.refusal;
    if (error !== null && error !== held.before) {
      refusal = error;
      sent = null;
    }
  });

  function raise(): void {
    if (target === null || !ready || sent !== null) return;
    refusal = null;
    const before = $belief.refusal;
    if (u.send(createBuilding(target, template, edited ? rules : null))) {
      sent = { addr: target, before };
    }
  }

  const fills = $derived(fillable(buildings, typed));
  const note = $derived.by(() => {
    switch (raising.kind) {
      case "blank":
        return say($lang, "raise_address_help");
      case "refused":
        return undefined;
      case "build":
        return say($lang, "raise_will_build");
      case "adopt":
        return say($lang, "raise_will_adopt");
    }
  });
</script>

<Page title={say($lang, "raise_title")} note={say($lang, "raise_note")} {rank}>
  <form
    class="flex min-w-0 flex-col gap-wide"
    onsubmit={(event) => {
      event.preventDefault();
      raise();
    }}
  >
    <div class="flex flex-col gap-snug" bind:this={addressGroup}>
      <Field
        label={say($lang, "raise_address")}
        value={typed}
        mono
        placeholder={say($lang, "raise_address_placeholder")}
        {...note === undefined ? {} : { help: note }}
        {...raising.kind === "refused" ? { error: say($lang, raising.why) } : {}}
        onInput={(value) => {
          typed = value;
        }}
      />
      {#if fills.length > 0}
        <div class="flex flex-wrap items-center gap-snug" role="group" aria-label={say($lang, "raise_waiting")}>
          <span class="text-note text-text-quiet">{say($lang, "raise_waiting")}</span>
          {#each fills as folder (folder)}
            <Button
              label={folder}
              onPress={() => {
                typed = folder;
                addressGroup?.querySelector("input")?.focus();
              }}
            />
          {/each}
        </div>
      {/if}
    </div>
    <div class="flex flex-col gap-snug">
      <span class="text-note text-text-quiet">{say($lang, "raise_template")}</span>
      <Segmented
        label={say($lang, "raise_template")}
        options={OFFERED.map((each) => ({ value: each, label: say($lang, `raise_template_${each}`) }))}
        held={template}
        onPick={(value) => {
          template = value;
        }}
      />
      <p class="text-note text-text-faint">{say($lang, `raise_template_${template}_note`)}</p>
    </div>
    {#if target !== null}
      <label class="flex flex-col gap-snug">
        <span class="text-note text-text-quiet">{say($lang, "raise_rules")}</span>
        <TextBox
          size="document"
          wire={textBoxWire(rules, undefined, refusal !== null, (value) => {
            rules = value;
            edited = true;
          })}
        />
        <span class="text-note text-text-faint">{say($lang, edited ? "raise_rules_edited" : "raise_rules_note")}</span>
      </label>
    {/if}
    {#if refusal !== null}
      <p class="text-note text-alert" role="alert">
        {fill(say($lang, "raise_refused"), { why: noticeTitle($lang, refusal.code, refusal.subject), fix: refusal.recovery })}
      </p>
    {/if}
    <div>
      <Button
        type="submit"
        tone="primary"
        loading={sent !== null}
        label={say($lang, raising.kind === "adopt" ? "raise_adopt" : "raise_build")}
        {...target === null
          ? { why: say($lang, "raise_needs_address") }
          : ready
            ? {}
            : { why: say($lang, "raise_rules_waiting") }}
      />
    </div>
  </form>
</Page>
