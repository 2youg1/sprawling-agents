// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A table of names and values: the `-e KEY=value` list a stdio server is
// started with, and the headers an http server is reached with.
//
// The table holds what a person wrote and nothing else. Whether a row
// can travel is `encode`'s question, so a table that a person fills in
// and the reason a send is refused stay two separate facts on the
// screen.

import { For, Show } from "solid-js";

import type { Pair } from "./draft";
import { Button } from "../parts/button";
import { Field } from "../parts/field";
import { useSay } from "../../ui";

export interface PairTableProps {
  // The heading, already in the person's language; it also names each
  // box, so two tables on one screen are told apart by a screen reader.
  readonly caption: string;
  readonly rows: readonly Pair[];
  readonly onChange: (rows: readonly Pair[]) => void;
  // What these rows are for, already in the person's language. Drawn
  // under the table, not in place of it.
  readonly note?: string;
}


export function PairTable(props: PairTableProps) {
  const say = useSay();
  const written = (at: number, pair: Pair) => {
    props.onChange(props.rows.map((row, index) => (index === at ? pair : row)));
  };
  return (
    <div class="flex w-full min-w-0 flex-col gap-tight">
      <span class="text-note text-text-quiet">{props.caption}</span>
      <For each={props.rows}>
        {(row, index) => (
          <div class="flex min-w-0 items-center gap-tight">
            <Field
              label={`${props.caption} ${say("mcp_pair_name")}`}
              labelling="hidden"
              mono
              placeholder={say("mcp_pair_name")}
              value={row.name}
              onInput={(value) => {
                written(index(), { name: value, value: row.value });
              }}
            />
            <Field
              label={`${props.caption} ${say("mcp_pair_value")}`}
              labelling="hidden"
              mono
              placeholder={say("mcp_pair_value")}
              value={row.value}
              onInput={(value) => {
                written(index(), { name: row.name, value });
              }}
            />
            <Button
              label={say("mcp_pair_remove")}
              tone="quiet"
              onPress={() => {
                props.onChange(props.rows.filter((_, at) => at !== index()));
              }}
            />
          </div>
        )}
      </For>
      <div class="flex items-center gap-base">
        <Button
          label={say("mcp_pair_add")}
          tone="quiet"
          onPress={() => {
            props.onChange([...props.rows, { name: "", value: "" }]);
          }}
        />
        <Show when={props.note}>
          {(note) => <span class="min-w-0 text-note text-text-faint">{note()}</span>}
        </Show>
      </div>
    </div>
  );
}
