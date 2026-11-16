<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
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
  import { fillable, OFFERED, raisingOf, sourceOf, sourceRefusal } from "./raise";

  interface Props {
    readonly address?: Address | undefined;

    readonly rank?: "page" | "section" | undefined;
  }

  const { address, rank = "page" }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const city = u.conn.asking.ask(QUERIES.city);

  // svelte-ignore state_referenced_locally
  let typed = $state<string>(address ?? "");

  let addressGroup = $state<HTMLDivElement>();
  let template = $state<Exclude<Template, "hall">>("minimal");
  let rules = $state("");

  let from = $state("");

  let edited = $state(false);

  let sent = $state.raw<{
    readonly addr: Address;
    readonly before: AxError | null;
    readonly importing: boolean;
  } | null>(null);
  let refusal = $state.raw<AxError | null>(null);

  const buildings = $derived($city !== undefined && "city" in $city ? $city.city.buildings : []);
  const raising = $derived(raisingOf(typed, buildings));
  const target = $derived(
    raising.kind === "build" || raising.kind === "adopt" ? raising.addr : null,
  );

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
    return u.conn.asking
      .ask({ template: { template: TemplateName.make(picked), addr: at } })
      .subscribe((answer) => {
        offered = answer !== undefined && "template" in answer ? answer.template.rules : null;
        answered = answer !== undefined;
      });
  });

  const ready = $derived(target !== null && (edited || answered));
  const importOf = $derived(sourceOf(from));

  const importing = $derived(sent?.importing ?? importOf !== null);
  const sourceWhy = $derived(sourceRefusal(from));
  $effect(() => {
    const text = offered;
    if (text !== null && !untrack(() => edited)) rules = text;
  });

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
    if (target === null || !ready || sent !== null || sourceWhy !== null) return;
    refusal = null;
    const before = $belief.refusal;
    if (u.send(createBuilding(target, template, edited ? rules : null, importOf))) {
      sent = { addr: target, before, importing: importOf !== null };

      u.conn.asking.refresh(QUERIES.city);
    }
  }

  const fills = $derived(fillable(buildings, typed));
  const note = $derived.by(() => {
    if (importOf !== null && sourceWhy === null) return say($lang, "raise_will_import");
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
        <div
          class="flex flex-wrap items-center gap-snug"
          role="group"
          aria-label={say($lang, "raise_waiting")}
        >
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

    <Field
      label={say($lang, "raise_source_label")}
      value={from}
      mono
      placeholder={say($lang, "raise_source_placeholder")}
      help={say($lang, "raise_source_help")}
      {...sourceWhy === null ? {} : { error: say($lang, sourceWhy) }}
      onInput={(value) => {
        from = value;
      }}
    />
    <div class="flex flex-col gap-snug">
      <span class="text-note text-text-quiet">{say($lang, "raise_template")}</span>
      <Segmented
        label={say($lang, "raise_template")}
        options={OFFERED.map((each) => ({
          value: each,
          label: say($lang, `raise_template_${each}`),
        }))}
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
        <span class="text-note text-text-faint"
          >{say($lang, edited ? "raise_rules_edited" : "raise_rules_note")}</span
        >
      </label>
    {/if}
    {#if refusal !== null}
      <p class="text-note text-alert" role="alert">
        {fill(say($lang, "raise_refused"), {
          why: noticeTitle($lang, refusal.code, refusal.subject),
          fix: refusal.recovery,
        })}
      </p>
    {/if}
    <div class="flex flex-col items-start gap-snug">
      <Button
        type="submit"
        tone="primary"
        loading={sent !== null}
        label={say(
          $lang,
          importing ? "raise_import" : raising.kind === "adopt" ? "raise_adopt" : "raise_build",
        )}
        {...sent !== null
          ? {}
          : target === null
            ? { why: say($lang, "raise_needs_address") }
            : sourceWhy !== null
              ? { why: say($lang, sourceWhy) }
              : ready
                ? {}
                : { why: say($lang, "raise_rules_waiting") }}
      />
      {#if sent !== null}
        <p class="text-note text-text-quiet" role="status">
          {say($lang, importing ? "raise_importing" : "raise_raising")}
        </p>
      {/if}
    </div>
  </form>
</Page>
