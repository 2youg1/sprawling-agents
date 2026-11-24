// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal } from "solid-js";
import { say } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import Button from "../../parts/button.solid";
import EmptyState from "../../parts/empty.solid";
import Table, { type Column } from "../../parts/table.solid";
import { GalleryCase } from "../case";
import { CHOSEN, MODELS, type ModelRow } from "../served";

export default function TableCases() {
  const lang = storeValue(ui().lang);
  const [rows, setRows] = createSignal<readonly ModelRow[]>(MODELS);
  const [picked, setPicked] = createSignal<readonly string[]>([CHOSEN.id]);
  const columns = createMemo<readonly Column<ModelRow>[]>(() => [
    {
      key: "id",
      header: say(lang(), "part_model_id"),
      render: (row) => <span class="font-mono text-note">{row.id}</span>,
      compare: (a, b) => a.id.localeCompare(b.id),
    },
    {
      key: "context",
      header: say(lang(), "part_context_window"),
      render: (row) => <span class="font-mono text-note">{row.context}</span>,
      compare: (a, b) => Number(a.context) - Number(b.context),
    },
    {
      key: "ceiling",
      header: say(lang(), "part_output_ceiling"),
      render: (row) => <span class="font-mono text-note">{row.ceiling}</span>,
      editable: {
        text: (row) => row.ceiling,
        onEdit: (row, ceiling) => {
          setRows((held) => held.map((each) => (each.id === row.id ? { ...each, ceiling } : each)));
        },
      },
    },
  ]);
  return (
    <>
      <GalleryCase label="table · partial selection, sortable, corrected in place">
        <Table
          caption={say(lang(), "setup_models")}
          columns={columns()}
          rows={rows()}
          keyOf={(row) => row.id}
          selection={{
            picked: (row) => picked().includes(row.id),
            onPick: (row, on) => {
              setPicked((held) => (on ? [...held, row.id] : held.filter((id) => id !== row.id)));
            },
            allLabel: say(lang(), "part_select_all"),
            allPicked: () => picked().length === rows().length,
            onPickAll: (on) => {
              setPicked(on ? rows().map((row) => row.id) : []);
            },
          }}
        />
      </GalleryCase>
      <GalleryCase label="table · empty">
        <Table
          caption={say(lang(), "setup_models")}
          columns={columns()}
          rows={[]}
          keyOf={(row) => row.id}
          empty={() => (
            <EmptyState
              missing="setup_no_models"
              action={<Button label={say(lang(), "setup_look")} tone="primary" />}
            />
          )}
        />
      </GalleryCase>
    </>
  );
}
