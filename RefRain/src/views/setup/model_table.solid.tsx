// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Show, type Accessor } from "solid-js";
import { readAnswer } from "../../core/answered";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Combobox from "../parts/combobox_view";
import Field from "../parts/field.solid";
import Table, { type Column } from "../parts/table.solid";
import { chosenRows, figuresQuery, freshTable } from "./model_rows";
import {
  lookOf,
  type ChoiceLook,
  type ColumnKey,
  type FigureLook,
  type ModelRowLook,
  type ModelTableLook,
} from "./model_table";
import type { ModelTableProps } from "./models";

export interface LiveModelTableProps extends ModelTableProps {
  readonly revision?: number | undefined;
}

function Choose(props: { readonly choice: ChoiceLook; readonly changed: () => void }) {
  return (
    <Combobox
      label={props.choice.label}
      placeholder={props.choice.placeholder}
      empty={props.choice.empty}
      choices={props.choice.choices}
      value={props.choice.value}
      onPick={(value) => {
        props.choice.pick(value);
        props.changed();
      }}
    />
  );
}

function FigureBox(props: { readonly box: FigureLook; readonly changed: () => void }) {
  return (
    <div class="flex flex-col gap-hair">
      <Field
        label={props.box.label}
        labelling="hidden"
        kind="number"
        step={1024}
        mono
        value={props.box.value}
        placeholder={props.box.placeholder}
        onInput={(value) => {
          props.box.input(value);
          props.changed();
        }}
      />
      <Show when={props.box.value === "" && props.box.source !== ""}>
        <span class="min-w-0 text-note text-text-faint">{props.box.source}</span>
      </Show>
    </div>
  );
}

function cellOf(key: ColumnKey, row: Accessor<ModelRowLook>, changed: () => void) {
  switch (key) {
    case "id":
      return (
        <div class="flex flex-col gap-hair">
          <span class="block whitespace-nowrap font-mono text-note text-text">{row().id}</span>
          <Show when={row().named}>
            {(name) => <span class="whitespace-nowrap text-note text-text-faint">{name()}</span>}
          </Show>
          <Show when={row().refusal}>
            {(why) => (
              <span role="alert" class="text-note text-alert">
                {why()}
              </span>
            )}
          </Show>
        </div>
      );
    case "context":
      return <FigureBox box={row().window} changed={changed} />;
    case "output":
      return <FigureBox box={row().ceiling} changed={changed} />;
    case "modalities":
      return (
        <div class="flex flex-col gap-hair">
          <Choose choice={row().input} changed={changed} />
          <Show when={row().said}>
            <span class="text-note text-text-faint">{row().said}</span>
          </Show>
        </div>
      );
    case "price":
      return (
        <span class="block whitespace-nowrap font-mono text-note text-text-faint">
          {row().price}
        </span>
      );
    case "role":
      return <Choose choice={row().role} changed={changed} />;
  }
}

export function ModelTableView(props: {
  readonly look: ModelTableLook;
  readonly changed: () => void;
}) {
  const columns = createMemo(() =>
    (props.look.table?.columns ?? []).map((column): Column<ModelRowLook> => ({
      key: column.key,
      header: column.header,
      min: column.min,
      renderReactive: (row) => cellOf(column.key, row, props.changed),
      ...(column.compare === undefined ? {} : { compare: column.compare }),
    })),
  );
  return (
    <div class="flex flex-col gap-snug text-note">
      <div class="flex items-center gap-snug">
        <div class="min-w-0 flex-1">
          <Field
            label={props.look.search.label}
            labelling="hidden"
            value={props.look.search.value}
            placeholder={props.look.search.placeholder}
            onInput={(value) => {
              props.look.search.input(value);
              props.changed();
            }}
          />
        </div>
        <label class="flex h-control shrink-0 cursor-pointer items-center gap-tight whitespace-nowrap rounded-control px-snug text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive">
          <input
            type="checkbox"
            data-control=""
            checked={props.look.textOnly.on}
            onChange={(event) => {
              props.look.textOnly.toggle(event.currentTarget.checked);
              props.changed();
            }}
          />
          {props.look.textOnly.label}
        </label>
      </div>
      <p class="text-text-faint">{props.look.count}</p>
      <Show when={props.look.table}>
        {(table) => (
          <Table
            caption={table().caption}
            columns={columns()}
            rows={table().rows}
            keyOf={(row) => row.id}
            empty={() => <p class="px-base py-snug text-text-faint">{table().empty}</p>}
            selection={{
              ...table().tick,
              onPick: (row, on) => {
                table().tick.onPick(row, on);
                props.changed();
              },
              onPickAll: (on) => {
                table().tick.onPickAll(on);
                props.changed();
              },
            }}
          />
        )}
      </Show>
      <Field
        label={props.look.manual.label}
        mono
        value={props.look.manual.value}
        placeholder={props.look.manual.placeholder}
        onInput={(value) => {
          props.look.manual.input(value);
          props.changed();
        }}
      />
    </div>
  );
}

export default function ModelTable(props: LiveModelTableProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const owned = freshTable();
  const [revision, setRevision] = createSignal(0);
  const table = () => {
    revision();
    void props.revision;
    return props.state ?? owned;
  };
  const chosen = createMemo(() => chosenRows(table(), props.served));
  createEffect(chosen, (rows) => {
    props.onChosen(rows);
  });

  const asked = storeValue(() => {
    const about = props.about;
    return about === null
      ? undefined
      : u.conn.asking.ask(figuresQuery(about, table(), props.served));
  });
  const figures = createMemo(() =>
    readAnswer(asked(), (answer) =>
      "model_figures" in answer ? answer.model_figures.models : undefined,
    ),
  );
  const look = createMemo(() => {
    const read = figures();
    return lookOf(table(), props.served, read.kind === "held" ? read.value : [], lang());
  });
  return <ModelTableView look={look()} changed={() => setRevision((held) => held + 1)} />;
}
