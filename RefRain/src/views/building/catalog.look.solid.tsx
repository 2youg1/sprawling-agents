// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { For, Match, Show, Switch } from "solid-js";
import Button from "../parts/button.solid";
import Segmented from "../parts/segmented.solid";
import type { CatalogLook, GroupLook, RowLook } from "./catalog_look";

function CatalogRow(props: { readonly row: RowLook }) {
  const row = () => props.row;
  return (
    <li class="flex min-w-0 flex-col border-t border-edge py-snug text-note">
      <div class="flex min-w-0 flex-wrap items-center gap-x-base gap-y-tight">
        <span class="flex min-w-0 grow basis-[16rem] flex-col gap-tight">
          <span class="flex min-w-0 items-baseline gap-base">
            <span class="shrink-0 font-mono text-text-quiet">{row().name}</span>
            <span class="min-w-0 truncate text-text-faint">{row().hint}</span>
            <Show when={row().change}>
              {(change) => <span class="shrink-0 text-alert">{change()}</span>}
            </Show>
          </span>
          <span class="min-w-0 text-text-faint">{row().facts}</span>
        </span>
        <Segmented
          label={row().label}
          options={row().choices}
          held={row().held}
          onPick={row().onPick}
        />
      </div>
      <Show when={row().actions.length > 0}>
        <div class="flex flex-wrap gap-base pt-snug">
          <For each={row().actions} keyed={(action) => action.key}>
            {(action) => {
              const disabled = () => {
                const why = action().why;
                return why === undefined ? {} : { why };
              };
              return (
                <Button
                  label={action().label}
                  tone="quiet"
                  {...disabled()}
                  onPress={action().onPress}
                />
              );
            }}
          </For>
        </div>
      </Show>
      <Show when={row().comparison}>
        {(comparison) => (
          <details class="pt-snug">
            <summary data-control="surface" class="cursor-pointer text-text-quiet">
              {row().change}
            </summary>
            <Show when={comparison().audit}>
              {(audit) => <p class="pt-tight text-text-faint">{audit()}</p>}
            </Show>
            <div class="grid min-w-0 grid-cols-2 gap-base pt-snug narrow:grid-cols-1">
              <pre class="min-w-0 overflow-auto whitespace-pre-wrap break-words rounded-card bg-page p-snug text-text-quiet">
                {comparison().pinned}
              </pre>
              <pre class="min-w-0 overflow-auto whitespace-pre-wrap break-words rounded-card bg-page p-snug text-text">
                {comparison().current}
              </pre>
            </div>
          </details>
        )}
      </Show>
    </li>
  );
}

function CatalogGroup(props: { readonly group: GroupLook }) {
  const group = () => props.group;
  return (
    <li class="rounded-panel bg-raised p-snug">
      <details open={group().open} class="flex min-w-0 flex-col gap-snug">
        <summary
          data-control="surface"
          class="flex min-w-0 cursor-pointer flex-wrap items-baseline gap-base text-note"
        >
          <span class="font-mono text-text">{group().title}</span>
          <span class="min-w-0 flex-1 text-text-faint">{group().summary}</span>
        </summary>
        <div class="flex min-w-0 flex-col gap-snug pt-snug">
          <Show when={group().rows.length > 0}>
            <Segmented
              label={group().summary}
              options={group().choices}
              held={group().held}
              onPick={group().onPick}
            />
          </Show>
          <ul class="flex min-w-0 flex-col">
            <For each={group().rows} keyed={(row) => row.key}>
              {(row) => <CatalogRow row={row()} />}
            </For>
          </ul>
        </div>
      </details>
    </li>
  );
}

export default function CatalogLookView(look: CatalogLook & { readonly rules?: JSX.Element }) {
  const copy = () => ((look.copy?.options.length ?? 0) > 0 ? look.copy : undefined);
  const blocked = () => {
    const why = look.save.why;
    return why === undefined ? {} : { why };
  };
  return (
    <div class="flex min-w-0 flex-col gap-wide">
      <div class="flex min-w-0 flex-col gap-snug">
        <Show when={look.scopes.length > 0}>
          <span class="text-note text-text-quiet">{look.scopeLabel}</span>
          <Segmented
            label={look.scopeLabel}
            options={look.scopes}
            held={look.scope}
            onPick={look.onScope}
          />
        </Show>
        <Segmented
          label={look.panesLabel}
          options={look.panes}
          held={look.pane}
          onPick={look.onPane}
        />
        <p class="text-note">
          <span class="figure text-text-quiet">{look.meter}</span>
          <Show when={look.delta}>
            {(delta) => <span class="figure text-text">· {delta()}</span>}
          </Show>
        </p>
        <Show when={look.beforeRun}>
          {(before) => <p class="text-note text-text-quiet">{before()}</p>}
        </Show>
        <div class="flex min-w-0 flex-wrap items-center gap-base">
          <Button
            label={look.unused.label}
            tone={look.unused.pressed ? "secondary" : "quiet"}
            onPress={look.unused.onPress}
          />
          <Show when={look.unused.pressed && look.groups.length > 0}>
            <Button label={look.unused.offLabel} tone="quiet" onPress={look.unused.onOff} />
          </Show>
          <Show when={copy()}>
            {(choices) => (
              <select
                class="h-control max-w-full rounded-control border border-edge-input bg-raised px-snug text-note text-text"
                aria-label={choices().label}
                data-control="surface"
                value=""
                onChange={(event) => {
                  const picked = choices().options.find(
                    (addr) => addr === event.currentTarget.value,
                  );
                  event.currentTarget.value = "";
                  if (picked !== undefined) choices().onPick(picked);
                }}
              >
                <option value="" disabled>
                  {choices().label}
                </option>
                <For each={choices().options} keyed>
                  {(addr) => <option value={addr}>{addr}</option>}
                </For>
              </select>
            )}
          </Show>
        </div>
        <Show when={look.openSessions}>
          {(sessions) => (
            <p class="text-note text-text-faint">
              {sessions().text}
              <a class="text-accent underline-offset-2 hover:underline" href={sessions().href}>
                {sessions().label}
              </a>
            </p>
          )}
        </Show>
      </div>
      <Show when={look.rulesNote}>
        {(note) => <p class="text-note text-text-quiet">{note()}</p>}
      </Show>
      <Switch
        fallback={
          <>
            <ul class="flex min-w-0 flex-col gap-snug">
              <For each={look.groups} keyed={(group) => group.key}>
                {(group) => <CatalogGroup group={group()} />}
              </For>
            </ul>
            <span class="flex flex-wrap items-center gap-base">
              <Button
                label={look.save.label}
                tone="primary"
                {...blocked()}
                onPress={look.save.onPress}
              />
            </span>
          </>
        }
      >
        <Match when={look.rules !== undefined}>{look.rules}</Match>
        <Match when={look.empty}>
          {(empty) => <p class="text-note text-text-faint">{empty()}</p>}
        </Match>
      </Switch>
    </div>
  );
}
