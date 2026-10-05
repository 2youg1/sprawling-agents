<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // What a run in this building may touch: the building's own sandbox,
  // as one card (client/Spec.lean §4-50, UG; `ConfigureBuilding.sandbox`).
  //
  // **Whole, because the city reads it whole.** A layer that speaks
  // about the sandbox speaks about all of it (`kernel::config::
  // SandboxLimits`), so the card shows every face and sends every face.
  // A building with no sandbox of its own runs under the city's; the
  // card says so and leaves the fuel box empty, because the default fuel
  // is a kernel constant the wire does not carry and the page will not
  // copy.
  //
  // **The grammar of each line is the wire's.** A mount is an `Address`,
  // a trusted connector a `ServerLabel`, both decoded through the
  // generated schemas; what those cannot see - a reserved subtree, a
  // name the city will not pass through - the city refuses, and the
  // refusal is a toast. The card calls itself saved only once the
  // city's answer holds what it sent.
  import { Option, Schema } from "effect";
  import { untrack } from "svelte";

  import { configureSandbox } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { ui } from "../../ui";
  import { Address, ContainerLimits, EnvVarName, ServerLabel } from "../../wire";
  import type { Interpreter, SandboxArm, SandboxLimits } from "../../wire";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import Segmented from "../parts/segmented.svelte";

  interface Props {
    readonly address: Address;
    // The building's own sandbox, or `null` when it has none.
    readonly held: SandboxLimits | null;
    // Whether the city has answered at all; before it has, nothing on
    // the card is anybody's value yet.
    readonly known: boolean;
  }

  const { address, held, known }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const SHELL = ["off", "on"] as const;
  const ARMS = ["default", "none", "copied_tree", "native", "container", "python"] as const;
  type ArmChoice = "default" | SandboxArm;
  const CONTAINER_FIELDS = ["image", "user", "cpu_millis", "memory_bytes", "pids"] as const;
  type ContainerField = (typeof CONTAINER_FIELDS)[number];

  // The three faces written one entry per line.
  type ListKey = "mounts" | "env" | "trusted";

  interface Draft {
    readonly shell: (typeof SHELL)[number];
    // Carried, not shown: a building chooses pwsh in its CONFIG.toml
    // (runtime D30), and saving the card must not undo that choice.
    readonly interpreter: Interpreter;
    readonly arm: ArmChoice;
    readonly containerFields: Readonly<Record<ContainerField, string>>;
    readonly fuel: string;
    readonly lists: Readonly<Record<ListKey, string>>;
  }

  function draftOf(limits: SandboxLimits | null): Draft {
    return {
      shell: limits?.shell === true ? "on" : "off",
      interpreter: limits?.interpreter ?? "system",
      arm: limits?.arm ?? (limits?.container == null ? "default" : "container"),
      containerFields: {
        image: limits?.container?.image ?? "",
        user: limits?.container == null ? "" : String(limits.container.user),
        cpu_millis: limits?.container == null ? "" : String(limits.container.cpu_millis),
        memory_bytes: limits?.container == null ? "" : String(limits.container.memory_bytes),
        pids: limits?.container == null ? "" : String(limits.container.pids),
      },
      fuel: limits === null ? "" : String(limits.fuel),
      lists: {
        mounts: (limits?.mounts ?? []).join("\n"),
        env: (limits?.env_passthrough ?? []).join("\n"),
        trusted: (limits?.trusted ?? []).join("\n"),
      },
    };
  }

  let draft = $state<Draft>(draftOf(null));
  let edited = $state(false);
  // What the last save sent, until the city's answer holds it.
  let sent = $state<string | null>(null);

  // The card follows the city's answer until somebody edits it, and
  // follows it again once their edit has landed.
  $effect(() => {
    const now = held;
    if (!untrack(() => edited)) draft = draftOf(now);
  });

  function lines(text: string): string[] {
    return text
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line !== "");
  }

  // Each list decoded line by line; the first line its schema refuses
  // is named, so the person knows which one to fix.
  function decoded<T>(text: string, schema: Schema.Codec<T, string>): { ok: T[] } | { bad: string } {
    const ok: T[] = [];
    for (const line of lines(text)) {
      const one = Schema.decodeOption(schema)(line);
      if (Option.isNone(one)) return { bad: line };
      ok.push(one.value);
    }
    return { ok };
  }

  const mounts = $derived(decoded(draft.lists.mounts, Address));
  const trusted = $derived(decoded(draft.lists.trusted, ServerLabel));
  const fuel = $derived(/^[1-9][0-9]*$/.test(draft.fuel.trim()) ? Number(draft.fuel.trim()) : null);

  const container = $derived(Schema.decodeOption(ContainerLimits)({
    image: draft.containerFields.image.trim(),
    user: Number(draft.containerFields.user),
    cpu_millis: Number(draft.containerFields.cpu_millis),
    memory_bytes: Number(draft.containerFields.memory_bytes),
    pids: Number(draft.containerFields.pids),
  }));
  const containerReady = $derived(draft.arm !== "container" || Option.isSome(container));

  const limits = $derived.by((): SandboxLimits | null =>
    fuel === null || !("ok" in mounts) || !("ok" in trusted) || !containerReady
      ? null
      : {
          shell: draft.shell === "on",
          interpreter: draft.interpreter,
          ...draft.arm === "default" ? {} : { arm: draft.arm },
          ...draft.arm === "container" && Option.isSome(container) ? { container: container.value } : {},
          fuel,
          mounts: mounts.ok,
          env_passthrough: lines(draft.lists.env).map((name) => EnvVarName.make(name)),
          trusted: trusted.ok,
        },
  );

  $effect(() => {
    if (sent !== null && held !== null && JSON.stringify(draftOf(held)) === sent) {
      sent = null;
      edited = false;
    }
  });

  const why = $derived.by((): string | undefined => {
    if (!known) return say($lang, "sandbox_unknown");
    if (fuel === null) return say($lang, "sandbox_fuel_needed");
    if (!containerReady) return say($lang, "sandbox_container_needed");
    if (!edited) return say($lang, "sandbox_unchanged");
    return undefined;
  });

  function edit(next: Partial<Draft>): void {
    draft = { ...draft, ...next };
    edited = true;
  }

  function editList(key: ListKey, value: string): void {
    edit({ lists: { ...draft.lists, [key]: value } });
  }

  // The three lists in the order the card draws them, each with the
  // line its schema refused, if any.
  const LISTS = $derived([
    { key: "mounts", label: "sandbox_mounts", help: "sandbox_mounts_help", bad: "bad" in mounts ? mounts.bad : null },
    { key: "env", label: "sandbox_env", help: "sandbox_env_help", bad: null },
    { key: "trusted", label: "sandbox_trusted", help: "sandbox_trusted_help", bad: "bad" in trusted ? trusted.bad : null },
  ] satisfies readonly { key: ListKey; label: Key; help: Key; bad: string | null }[]);

  function save(): void {
    if (limits !== null && u.send(configureSandbox(address, limits))) {
      sent = JSON.stringify(draftOf(limits));
    }
  }
</script>

<section class="flex max-w-page min-w-0 flex-col gap-base" aria-label={say($lang, "bld_sandbox")}>
  <h2 class="text-note text-text-faint">{say($lang, "bld_sandbox")}</h2>
  <p class="max-w-measure text-note text-text-quiet">
    {held === null && known ? say($lang, "sandbox_inherited") : say($lang, "sandbox_what")}
  </p>
  <div class="grid grid-cols-[repeat(auto-fit,minmax(min(100%,var(--container-measure)),1fr))] gap-x-gutter gap-y-wide border-y border-edge py-wide">
    <div class="flex min-w-0 flex-col gap-snug">
      <span class="text-note text-text-quiet">{say($lang, "sandbox_shell")}</span>
      <Segmented
        label={say($lang, "sandbox_shell")}
        options={SHELL.map((value) => ({ value, label: say($lang, `sandbox_shell_${value}`) }))}
        held={draft.shell}
        onPick={(shell: (typeof SHELL)[number]) => {
          edit({ shell });
        }}
      />
      <p class="text-note text-text-faint">{say($lang, "sandbox_shell_help")}</p>
    </div>
    <div class="flex min-w-0 flex-col gap-snug">
      <span class="text-note text-text-quiet">{say($lang, "sandbox_arm")}</span>
      <select
        class="h-control w-full rounded-control border border-edge-input bg-raised px-base text-note text-text"
        aria-label={say($lang, "sandbox_arm")}
        value={draft.arm}
        onchange={(event) => {
          const arm = ARMS.find((each) => each === event.currentTarget.value);
          if (arm !== undefined) edit({ arm });
        }}
      >
        {#each ARMS as arm (arm)}
          <option value={arm}>{say($lang, `sandbox_arm_${arm}`)}</option>
        {/each}
      </select>
      <p class="text-note text-text-faint">{say($lang, "sandbox_arm_help")}</p>
      {#if draft.arm === "container"}
        <p class="text-note text-text-faint">{say($lang, "sandbox_container_help")}</p>
      {/if}
    </div>
    {#if draft.arm === "container"}
      {#each CONTAINER_FIELDS as field (field)}
        <Field label={say($lang, `sandbox_container_${field}`)} mono
          kind={field === "image" ? "text" : "number"} step={1}
          value={draft.containerFields[field]}
          onInput={(value) => { edit({ containerFields: { ...draft.containerFields, [field]: value } }); }} />
      {/each}
    {/if}
    <Field
      label={say($lang, "sandbox_fuel")}
      help={say($lang, "sandbox_fuel_help")}
      kind="number"
      step={1}
      mono
      value={draft.fuel}
      onInput={(value) => {
        edit({ fuel: value });
      }}
    />
    {#each LISTS as list (list.key)}
      <label class="flex min-w-0 flex-col gap-snug">
        <span class="text-note text-text-quiet">{say($lang, list.label)}</span>
        <textarea
          class="min-h-[calc(var(--spacing-control)*3)] w-full rounded-control border border-edge-input bg-raised px-base py-snug font-mono text-note text-text aria-invalid:border-alert"
          aria-invalid={list.bad !== null}
          value={draft.lists[list.key]}
          oninput={(event) => {
            editList(list.key, event.currentTarget.value);
          }}
        ></textarea>
        {#if list.bad === null}
          <span class="text-note text-text-faint">{say($lang, list.help)}</span>
        {:else}
          <span class="text-note text-alert" role="alert">{fill(say($lang, "sandbox_bad_line"), { line: list.bad })}</span>
        {/if}
      </label>
    {/each}
  </div>
  <div class="flex items-center gap-base">
    <Button label={say($lang, "sandbox_save")} tone="primary" loading={sent !== null} {...why === undefined ? {} : { why }} onPress={save} />
    {#if sent === null && !edited && held !== null}
      <span class="text-note text-text-faint" role="status">{say($lang, "sandbox_saved")}</span>
    {/if}
  </div>
</section>
