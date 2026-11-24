// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";

import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import Button from "../../parts/button.solid";
import Field from "../../parts/field.solid";
import IconButton from "../../parts/icon_button.solid";
import Segmented from "../../parts/segmented.solid";
import { HALL } from "../../shared/buildings";
import { lookOf, type BoxLook, type PairTableLook, type SetDraft } from "./advanced";
import type { Draft } from "./draft";
import "./advanced_view.css";

interface AdvancedProps {
  readonly draft: Draft;
  readonly setDraft: SetDraft;
  readonly onRenamed: () => void;
}

function Box(props: { readonly field: BoxLook }) {
  return (
    <Field
      label={props.field.label}
      kind={props.field.kind}
      mono={props.field.mono}
      value={props.field.value}
      placeholder={props.field.placeholder}
      onInput={props.field.input}
      {...(props.field.help === undefined ? {} : { help: props.field.help })}
      {...(props.field.step === undefined ? {} : { step: props.field.step })}
      {...(props.field.pattern === undefined ? {} : { pattern: props.field.pattern })}
    />
  );
}

function Pairs(props: { readonly table: PairTableLook }) {
  return (
    <div class="flex flex-col gap-tight text-note text-text-quiet">
      {props.table.title}
      <For each={props.table.rows} keyed={(row) => row.key}>
        {(row) => (
          <div class="flex items-center gap-tight">
            <Field
              label={row().name.label}
              labelling="hidden"
              mono
              value={row().name.value}
              onInput={row().name.input}
            />
            <Field
              label={row().value.label}
              labelling="hidden"
              mono
              value={row().value.value}
              onInput={row().value.input}
            />
            <IconButton label={row().remove.label} glyph="cross" onPress={row().remove.press} />
          </div>
        )}
      </For>
      <Button
        label={props.table.add.label}
        tone="quiet"
        glyph="add"
        onPress={props.table.add.press}
      />
    </div>
  );
}

export default function Advanced(props: AdvancedProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const config = storeValue(u.conn.asking.ask({ config: { addr: HALL } }));
  const look = createMemo(() => {
    const answer = config();
    const defaults = answer !== undefined && "config" in answer ? answer.config.tuning : undefined;
    return lookOf(props.draft, defaults, lang(), {
      setDraft: props.setDraft,
      onRenamed: props.onRenamed,
    });
  });
  return (
    <details class="flex flex-col gap-snug rounded-card bg-chrome px-base py-snug">
      <summary
        class="w-fit cursor-pointer rounded-control px-snug py-tight text-label text-text-quiet"
        data-control="surface"
      >
        {look().title}
      </summary>
      <div class="provider-fields grid gap-snug">
        <For each={look().naming} keyed={(field) => field.key}>
          {(field) => <Box field={field()} />}
        </For>
      </div>
      <div class="provider-fields grid gap-snug">
        <For each={look().tuning} keyed={(field) => field.key}>
          {(field) => <Box field={field()} />}
        </For>
      </div>
      <div class="flex flex-col gap-tight text-note text-text-quiet">
        {look().proxying.label}
        <Segmented
          label={look().proxying.label}
          options={look().proxying.options}
          held={look().proxying.held}
          onPick={look().proxying.pick}
        />
        <span class="text-text-faint">{look().proxying.help}</span>
        <Show when={look().proxying.note}>
          <span class="text-text-faint">{look().proxying.note}</span>
        </Show>
      </div>
      <For each={look().tables} keyed={(table) => table.key}>
        {(table) => <Pairs table={table()} />}
      </For>
    </details>
  );
}
