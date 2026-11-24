// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";
import { createEffect, createMemo, createSignal, For, Show } from "solid-js";

import { configureSandbox } from "../../core/commands";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { ContainerLimits, EnvVarName, ServerLabel } from "../../wire";
import type { Address, Interpreter, SandboxArm, SandboxLimits } from "../../wire";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
import Segmented from "../parts/segmented.solid";
import { armSelectWire } from "./arm_select";
import ArmSelect from "./arm_select.look.solid";
import { textBoxWire } from "./text_box";
import TextBox from "./text_box.look.solid";

interface Props {
  readonly address: Address;
  readonly held: SandboxLimits | null;
  readonly known: boolean;
}

const SHELL = ["off", "on"] as const;
const ARMS = ["default", "none", "copied_tree", "native", "container", "python"] as const;
type ArmChoice = "default" | SandboxArm;
const CONTAINER_FIELDS = ["image", "user", "cpu_millis", "memory_bytes", "pids"] as const;
type ContainerField = (typeof CONTAINER_FIELDS)[number];

type ListKey = "env" | "trusted";

interface Draft {
  readonly shell: (typeof SHELL)[number];

  readonly interpreter: Interpreter;
  readonly arm: ArmChoice;
  readonly containerFields: Readonly<Record<ContainerField, string>>;
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
    lists: {
      env: (limits?.env_passthrough ?? []).join("\n"),
      trusted: (limits?.trusted ?? []).join("\n"),
    },
  };
}

function lines(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

function decoded<T>(text: string, schema: Schema.Codec<T, string>): { ok: T[] } | { bad: string } {
  const ok: T[] = [];
  for (const line of lines(text)) {
    const one = Schema.decodeOption(schema)(line);
    if (Option.isNone(one)) return { bad: line };
    ok.push(one.value);
  }
  return { ok };
}

export default function Sandbox(props: Props) {
  const u = ui();
  const lang = storeValue(u.lang);
  const [draft, setDraft] = createSignal<Draft>(draftOf(null));
  const [edited, setEdited] = createSignal(false);
  const [sent, setSent] = createSignal<string | null>(null);
  const env = createMemo(() => decoded(draft().lists.env, EnvVarName));
  const trusted = createMemo(() => decoded(draft().lists.trusted, ServerLabel));
  const container = createMemo(() => {
    const fields = draft().containerFields;
    return Schema.decodeOption(ContainerLimits)({
      image: fields.image.trim(),
      user: Number(fields.user),
      cpu_millis: Number(fields.cpu_millis),
      memory_bytes: Number(fields.memory_bytes),
      pids: Number(fields.pids),
    });
  });
  const containerReady = () => draft().arm !== "container" || Option.isSome(container());
  const selected = createMemo((): SandboxLimits | null => {
    const environment = env();
    const servers = trusted();
    const box = container();
    const d = draft();
    if (!("ok" in environment) || !("ok" in servers) || !containerReady()) return null;
    return {
      shell: d.shell === "on",
      interpreter: d.interpreter,
      ...(d.arm === "default" ? {} : { arm: d.arm }),
      ...(d.arm === "container" && Option.isSome(box) ? { container: box.value } : {}),
      env_passthrough: environment.ok,
      trusted: servers.ok,
    };
  });

  createEffect(
    () => props.held,
    (held) => {
      const incoming = draftOf(held);
      const waiting = sent();
      if (waiting !== null && held !== null && JSON.stringify(incoming) === waiting) {
        setSent(null);
        const written = selected();
        if (written !== null && JSON.stringify(draftOf(written)) === waiting) {
          setEdited(false);
          setDraft(incoming);
        }
      } else if (!edited()) {
        setDraft(incoming);
      }
    },
  );

  const why = (): { readonly why?: string } => {
    if (!props.known) return { why: say(lang(), "sandbox_unknown") };
    if (!containerReady()) return { why: say(lang(), "sandbox_container_needed") };
    const environment = env();
    if ("bad" in environment)
      return { why: fill(say(lang(), "sandbox_bad_line"), { line: environment.bad }) };
    const servers = trusted();
    if ("bad" in servers)
      return { why: fill(say(lang(), "sandbox_bad_line"), { line: servers.bad }) };
    return edited() ? {} : { why: say(lang(), "sandbox_unchanged") };
  };

  function edit(next: Partial<Draft>): void {
    setDraft((current) => ({ ...current, ...next }));
    setEdited(true);
  }

  const lists = [
    { key: "env", label: "sandbox_env", help: "sandbox_env_help", validation: env },
    { key: "trusted", label: "sandbox_trusted", help: "sandbox_trusted_help", validation: trusted },
  ] as const;

  function save(): void {
    const limits = selected();
    if (limits !== null && u.send(configureSandbox(props.address, limits))) {
      setSent(JSON.stringify(draftOf(limits)));
    }
  }

  return (
    <section
      class="flex max-w-page min-w-0 flex-col gap-base"
      aria-label={say(lang(), "bld_sandbox")}
    >
      <h2 class="sr-only">{say(lang(), "bld_sandbox")}</h2>
      <p class="max-w-measure text-note text-text-quiet">
        {say(lang(), props.held === null && props.known ? "sandbox_inherited" : "sandbox_what")}
      </p>
      <div class="grid grid-cols-[repeat(auto-fit,minmax(min(100%,var(--container-measure)),1fr))] gap-x-gutter gap-y-wide border-y border-edge py-wide">
        <div class="flex min-w-0 flex-col gap-snug">
          <span class="text-note text-text-quiet">{say(lang(), "sandbox_shell")}</span>
          <Segmented
            label={say(lang(), "sandbox_shell")}
            options={SHELL.map((value) => ({
              value,
              label: say(lang(), `sandbox_shell_${value}`),
            }))}
            held={draft().shell}
            onPick={(shell) => edit({ shell })}
          />
          <p class="text-note text-text-faint">{say(lang(), "sandbox_shell_help")}</p>
        </div>
        <div class="flex min-w-0 flex-col gap-snug">
          <span class="text-note text-text-quiet">{say(lang(), "sandbox_arm")}</span>
          <ArmSelect
            options={ARMS.map((arm) => ({ value: arm, label: say(lang(), `sandbox_arm_${arm}`) }))}
            wire={armSelectWire(say(lang(), "sandbox_arm"), draft().arm, (value) => {
              const arm = ARMS.find((each) => each === value);
              if (arm !== undefined) edit({ arm });
            })}
          />
          <p class="text-note text-text-faint">{say(lang(), "sandbox_arm_help")}</p>
          <Show when={draft().arm === "container"}>
            <p class="text-note text-text-faint">{say(lang(), "sandbox_container_help")}</p>
          </Show>
        </div>
        <Show when={draft().arm === "container"}>
          <For each={CONTAINER_FIELDS} keyed>
            {(field) => (
              <Field
                label={say(lang(), `sandbox_container_${field}`)}
                mono
                kind={field === "image" ? "text" : "number"}
                step={1}
                value={draft().containerFields[field]}
                onInput={(value) =>
                  edit({ containerFields: { ...draft().containerFields, [field]: value } })
                }
              />
            )}
          </For>
        </Show>
        <For each={lists}>
          {(list) => {
            const bad = () => {
              const held = list.validation();
              return "bad" in held ? held.bad : null;
            };
            return (
              <label class="flex min-w-0 flex-col gap-snug">
                <span class="text-note text-text-quiet">{say(lang(), list.label)}</span>
                <TextBox
                  size="list"
                  wire={textBoxWire(draft().lists[list.key], undefined, bad() !== null, (value) => {
                    edit({ lists: { ...draft().lists, [list.key]: value } });
                  })}
                />
                <Show
                  when={bad()}
                  fallback={<span class="text-note text-text-faint">{say(lang(), list.help)}</span>}
                >
                  {(line) => (
                    <span class="text-note text-alert" role="alert">
                      {fill(say(lang(), "sandbox_bad_line"), { line: line() })}
                    </span>
                  )}
                </Show>
              </label>
            );
          }}
        </For>
      </div>
      <div class="flex items-center gap-base">
        <Button
          label={say(lang(), "sandbox_save")}
          tone="primary"
          loading={sent() !== null}
          {...why()}
          onPress={save}
        />
        <Show when={sent() === null && !edited() && props.held !== null}>
          <span class="text-note text-text-faint" role="status">
            {say(lang(), "sandbox_saved")}
          </span>
        </Show>
      </div>
    </section>
  );
}
