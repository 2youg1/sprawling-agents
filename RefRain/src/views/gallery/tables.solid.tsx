// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal } from "solid-js";
import { fill, say } from "../../core/lang";
import { STOP } from "../../core/slash";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Badge from "../parts/badge.solid";
import Button from "../parts/button.solid";
import EmptyState from "../parts/empty.solid";
import Path from "../parts/path.solid";
import Row, { RowList } from "../parts/row.solid";
import Table, { type Column } from "../parts/table.solid";
import { GalleryCase } from "./case";
interface Served {
  readonly id: string;
  readonly context: string;
  readonly ceiling: string;
  readonly modalities: string;
  readonly price: string;
  readonly role: string;
}

const LONG = "acme-inference/orion-3-ultra-preview-2026-09";

const SERVED: readonly Served[] = [
  {
    id: LONG,
    context: "1024000",
    ceiling: "65536",
    modalities: "image text",
    price: "0.000003 / 0.000015",
    role: "main",
  },
  {
    id: "openai/gpt-nucleus-6",
    context: "400000",
    ceiling: "—",
    modalities: "text",
    price: "—",
    role: "—",
  },
  {
    id: "meta/muse-spark-1.3-contributor",
    context: "131072",
    ceiling: "8192",
    modalities: "text",
    price: "0.000001 / 0.000004",
    role: "digest",
  },
];

function keyOf(row: Served): string {
  return row.id;
}

const still = (): void => undefined;

export default function TableWidthCases() {
  const lang = storeValue(ui().lang);
  const [picked, setPicked] = createSignal<readonly string[]>([LONG]);
  const columns = createMemo<readonly Column<Served>[]>(() => [
    {
      key: "id",
      header: say(lang(), "part_model_id"),
      min: "24ch",
      render: (row) => <span class="block whitespace-nowrap font-mono text-note">{row.id}</span>,
      compare: (a, b) => a.id.localeCompare(b.id),
    },
    {
      key: "context",
      header: say(lang(), "part_context_window"),
      min: "figure",
      render: (row) => <span class="font-mono text-note">{row.context}</span>,
      compare: (a, b) => Number(a.context) - Number(b.context),
    },
    {
      key: "ceiling",
      header: say(lang(), "part_output_ceiling"),
      min: "figure",
      render: (row) => <span class="font-mono text-note">{row.ceiling}</span>,
      compare: (a, b) => Number(a.ceiling) - Number(b.ceiling),
    },
    {
      key: "modalities",
      header: say(lang(), "setup_model_modalities"),
      min: "12ch",
      render: (row) => <span class="text-note text-text-faint">{row.modalities}</span>,
    },
    {
      key: "price",
      header: say(lang(), "setup_model_price"),
      min: "12ch",
      render: (row) => (
        <span class="block whitespace-nowrap font-mono text-note text-text-faint">{row.price}</span>
      ),
    },
    {
      key: "role",
      header: say(lang(), "setup_model_role"),
      min: "figure",
      render: (row) => <span class="font-mono text-note">{row.role}</span>,
    },
  ]);
  return (
    <>
      <GalleryCase label="table · a 24ch id column inside a 320px container" width={320}>
        <div class="min-w-0">
          <Table
            caption={say(lang(), "setup_models")}
            columns={columns()}
            rows={SERVED}
            keyOf={keyOf}
          />
        </div>
      </GalleryCase>
      <GalleryCase label="table · six columns inside 520px, one row ticked" width={520}>
        <div class="min-w-0">
          <Table
            caption={say(lang(), "setup_models")}
            columns={columns()}
            rows={SERVED}
            keyOf={keyOf}
            selection={{
              picked: (row) => picked().includes(row.id),
              onPick: (row, on) => {
                setPicked((values) =>
                  on ? [...values, row.id] : values.filter((id) => id !== row.id),
                );
              },
              allLabel: say(lang(), "setup_select_all"),
              allPicked: () => picked().length === SERVED.length,
              onPickAll: (on) => {
                setPicked(on ? SERVED.map(keyOf) : []);
              },
            }}
          />
        </div>
      </GalleryCase>
      <GalleryCase label="path · an address too long for a 240px column" width={240}>
        <div class="min-w-0 rounded-card border border-edge-panel px-base py-snug">
          <Path path="city/lab/west/experiments/2026/07/completions-with-a-long-name.md" />
        </div>
      </GalleryCase>
      <GalleryCase label="rows · a long name, a second line and a right-hand action">
        <div class="rounded-card border border-edge-panel">
          <RowList label={say(lang(), "rec_ledger")}>
            <Row
              primary={LONG}
              secondary={fill(say(lang(), "talk_tokens"), { n: "12480" })}
              onOpen={still}
              status={<Badge text={say(lang(), "status_in_progress")} weight="live" dot />}
              actions={<Button label={STOP} tone="quiet" />}
            />
            <Row
              primary="openai/gpt-nucleus-6"
              secondary={say(lang(), "tree_no_runs")}
              status={<Badge text={say(lang(), "city_quiet")} dot />}
            />
          </RowList>
        </div>
      </GalleryCase>
      <GalleryCase label="rows · nothing in the list">
        <div class="rounded-card border border-edge-panel">
          <EmptyState
            missing="rec_nothing"
            action={<Button label={say(lang(), "city_ask_mayor")} tone="primary" />}
          />
        </div>
      </GalleryCase>
    </>
  );
}
