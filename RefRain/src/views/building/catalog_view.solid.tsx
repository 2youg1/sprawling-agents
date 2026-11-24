// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch } from "solid-js";
import { storeValue } from "../../core/solid_store";
import type { Answered } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { kilo } from "../../core/time";
import { ui } from "../../ui";
import type { Address, Effort, Query } from "../../wire";
import { TIERS } from "./subagent";
import type { Tier } from "./subagent";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
import Unanswered from "../parts/unanswered.solid";
import { rulesAt } from "../settings/files";
import { edited as markEdited } from "../settings/saving";
import type { Saving } from "../settings/saving";
import { textBoxWire } from "./text_box";
import TextBox from "./text_box.look.solid";
import Look from "./catalog.look.solid";
import type { CatalogLook, CatalogPane } from "./catalog_look";
interface Endpoint {
  name: string;
  label: string;
  models: readonly { id: string; thinking: { levels: readonly Effort[] } }[];
}
export interface CatalogViewProps {
  building: Address;
  read: Answered<unknown>;
  look: CatalogLook | undefined;
  pane: CatalogPane;
  saved: boolean;
  question: Query;
  rulesFailed: { query: string; reason: string | null } | null;
  onDisk: string | null;
  rulesDraft: string;
  rulesSaving: Saving;
  endpointRead: Answered<unknown>;
  endpointList: readonly Endpoint[];
  providerModels: readonly { id: string; thinking: { levels: readonly Effort[] } }[];
  levels: readonly Effort[];
  shortcuts: Partial<Record<Tier, { endpoint: string; model: string; effort: Effort }>>;
  choiceBlocked: string | undefined;
  provider: string;
  modelId: string;
  level: Effort;
  lockBox: string;
  lockEdited: boolean;
  lockField: { kind: "clear" } | { kind: "set"; tokens: number } | { kind: "invalid" };
  modelWindow: number | null;
  share: number | null;
  saveRules: () => void;
  saveLock: () => void;
  saveChoice: () => void;
  useTier: (tier: Tier) => void;
  setRulesDraft: (next: string) => void;
  setRulesSaving: (next: Saving) => void;
  setProvider: (next: string) => void;
  setModelId: (next: string) => void;
  setLevel: (next: Effort) => void;
  setChoiceDirty: (next: boolean) => void;
  setLockBox: (next: string) => void;
  setLockEdited: (next: boolean) => void;
}
const SELECT =
  "h-control max-w-full rounded-control border border-edge-input bg-raised px-snug text-note text-text";

export default function CatalogView(props: CatalogViewProps) {
  const lang = storeValue(ui().lang);
  const unavailable = () => (props.read.kind === "unavailable" ? props.read : undefined);
  const endpointFailed = () =>
    props.endpointRead.kind === "unavailable" ? props.endpointRead : undefined;

  function ResultWord() {
    const recovery = () => {
      const held = props.rulesSaving;
      return held.kind === "refused" ? held.error.recovery : undefined;
    };
    return (
      <Switch>
        <Match when={props.rulesSaving.kind === "saved"}>
          <span class="text-note text-text-quiet" role="status">
            {say(lang(), "catalog_saved")}
          </span>
        </Match>
        <Match when={recovery()}>
          {(message) => (
            <span class="text-note text-text-quiet" role="status">
              {message()}
            </span>
          )}
        </Match>
      </Switch>
    );
  }

  function ModelChoice() {
    return (
      <Switch>
        <Match when={endpointFailed()}>
          {(held) => (
            <Unanswered query={held().query} reason={held().reason} asked={QUERIES.endpoints} />
          )}
        </Match>
        <Match when={props.endpointRead.kind === "asking"}>
          <p class="text-text-faint">…</p>
        </Match>
        <Match when={props.endpointList.length === 0}>
          <p class="text-note text-text-faint">{say(lang(), "subagent_no_endpoint")}</p>
        </Match>
        <Match when={true}>
          <div class="flex min-w-0 flex-wrap items-end gap-base">
            <label class="flex min-w-0 flex-col gap-tight text-note">
              <span class="text-text-quiet">{say(lang(), "subagent_provider")}</span>
              <select
                class={SELECT}
                data-control="surface"
                value={props.provider}
                onChange={(event) => {
                  const name = event.currentTarget.value;
                  const first = props.endpointList.find((endpoint) => endpoint.name === name)
                    ?.models[0];
                  props.setProvider(name);
                  props.setModelId(first?.id ?? "");
                  props.setLevel(first?.thinking.levels[0] ?? "medium");
                  props.setChoiceDirty(true);
                }}
              >
                <option value="" />
                <For each={props.endpointList} keyed={(endpoint) => endpoint.name}>
                  {(endpoint) => <option value={endpoint().name}>{endpoint().label}</option>}
                </For>
              </select>
            </label>
            <label class="flex min-w-0 flex-col gap-tight text-note">
              <span class="text-text-quiet">{say(lang(), "subagent_model")}</span>
              <select
                class={SELECT}
                data-control="surface"
                value={props.modelId}
                onChange={(event) => {
                  const id = event.currentTarget.value;
                  props.setModelId(id);
                  const offered =
                    props.providerModels.find((each) => each.id === id)?.thinking.levels ?? [];
                  const first = offered[0];
                  if (first === undefined) props.setLevel("medium");
                  else if (!offered.includes(props.level)) props.setLevel(first);
                  props.setChoiceDirty(true);
                }}
              >
                <For each={props.providerModels} keyed={(model) => model.id}>
                  {(model) => <option value={model().id}>{model().id}</option>}
                </For>
              </select>
            </label>
            <label class="flex min-w-0 flex-col gap-tight text-note">
              <span class="text-text-quiet">{say(lang(), "subagent_effort")}</span>
              <select
                class={SELECT}
                data-control="surface"
                value={props.level}
                onChange={(event) => {
                  const next = props.levels.find((each) => each === event.currentTarget.value);
                  if (next !== undefined) {
                    props.setLevel(next);
                    props.setChoiceDirty(true);
                  }
                }}
              >
                <For each={props.levels} keyed>
                  {(effort) => <option value={effort}>{effort}</option>}
                </For>
              </select>
            </label>
          </div>
          <Show when={props.modelId !== "" && props.levels.length === 0}>
            <p class="text-note text-text-faint">{say(lang(), "subagent_no_effort")}</p>
          </Show>
          <span class="flex flex-wrap items-center gap-base">
            <For each={TIERS} keyed>
              {(tier) => (
                <Show when={props.shortcuts[tier]}>
                  <Button
                    label={fill(say(lang(), "subagent_shortcut"), { tier })}
                    tone="quiet"
                    onPress={() => props.useTier(tier)}
                  />
                </Show>
              )}
            </For>
            <Button
              label={say(lang(), "subagent_save")}
              tone="primary"
              loading={props.rulesSaving.kind === "saving"}
              {...(props.choiceBlocked === undefined ? {} : { why: props.choiceBlocked })}
              onPress={props.saveChoice}
            />
            <ResultWord />
          </span>
        </Match>
      </Switch>
    );
  }

  function Rules() {
    return (
      <div class="flex min-w-0 flex-col gap-base">
        <Switch>
          <Match when={props.rulesFailed}>
            {(failure) => (
              <Unanswered
                query={failure().query}
                reason={failure().reason}
                asked={{ document: { at: rulesAt(props.building) } }}
              />
            )}
          </Match>
          <Match when={props.onDisk === null}>
            <p class="text-text-faint">…</p>
          </Match>
          <Match when={true}>
            <Show when={props.pane === "models"}>
              <ModelChoice />
            </Show>
            <TextBox
              size="document"
              wire={textBoxWire(props.rulesDraft, say(lang(), "rules_title"), false, (next) => {
                props.setRulesDraft(next);
                props.setRulesSaving(markEdited(next !== props.onDisk));
              })}
            />
            <span class="flex flex-wrap items-center gap-base">
              <Button
                label={say(lang(), "catalog_save_rules")}
                tone="primary"
                loading={props.rulesSaving.kind === "saving"}
                {...(props.rulesDraft === props.onDisk
                  ? { why: say(lang(), "catalog_unchanged") }
                  : {})}
                onPress={props.saveRules}
              />
              <ResultWord />
            </span>
          </Match>
        </Switch>
      </div>
    );
  }

  function ContextLock() {
    const blocked = () => {
      if (props.lockField.kind === "invalid")
        return { why: say(lang(), "catalog_context_lock_invalid") };
      if (props.rulesSaving.kind === "saving") return { why: say(lang(), "subagent_saving") };
      return props.lockEdited ? {} : { why: say(lang(), "catalog_unchanged") };
    };
    return (
      <div class="flex min-w-0 flex-col gap-base">
        <Field
          label={say(lang(), "catalog_context_lock")}
          kind="number"
          step={1}
          suffix={say(lang(), "catalog_context_lock_unit")}
          mono
          placeholder={say(lang(), "catalog_context_lock_hint")}
          value={props.lockBox}
          onInput={(next) => {
            props.setLockBox(next);
            props.setLockEdited(true);
          }}
        />
        <Show
          when={props.modelWindow}
          fallback={
            <p class="text-note text-text-faint">{say(lang(), "catalog_context_window_unknown")}</p>
          }
        >
          {(window) => (
            <p class="text-note text-text-quiet">
              {fill(say(lang(), "catalog_context_window"), { window: kilo(window()) })}
              <Show when={props.share === null ? undefined : { share: props.share }}>
                {(part) => (
                  <> · {fill(say(lang(), "catalog_context_share"), { n: String(part().share) })}</>
                )}
              </Show>
            </p>
          )}
        </Show>
        <p class="text-note text-text-faint">
          {say(lang(), "catalog_context_warm")}
          <a
            class="text-accent underline-offset-2 hover:underline"
            href={toFragment({ kind: "setup", group: "accounts" })}
          >
            {say(lang(), "nav_settings")}
          </a>
        </p>
        <span class="flex flex-wrap items-center gap-base">
          <Button
            label={say(lang(), "catalog_context_save")}
            tone="primary"
            loading={props.rulesSaving.kind === "saving"}
            {...blocked()}
            onPress={props.saveLock}
          />
          <ResultWord />
        </span>
      </div>
    );
  }

  return (
    <section
      class="flex max-w-page min-w-0 flex-col gap-wide"
      aria-label={say(lang(), "bld_catalog")}
    >
      <h2 class="sr-only">{say(lang(), "bld_catalog")}</h2>
      <Switch fallback={<p class="text-text-faint">…</p>}>
        <Match when={unavailable()}>
          {(held) => <Unanswered query={held().query} asked={props.question} />}
        </Match>
        <Match when={props.look}>
          {(look) => (
            <>
              <Look
                {...look()}
                rules={
                  props.pane === "models" || props.pane === "rules" ? (
                    <Rules />
                  ) : props.pane === "context" ? (
                    <ContextLock />
                  ) : undefined
                }
              />
              <Show when={props.saved}>
                <p class="text-note text-text-quiet" role="status">
                  {say(lang(), "catalog_saved")}
                </p>
              </Show>
            </>
          )}
        </Match>
      </Switch>
    </section>
  );
}
