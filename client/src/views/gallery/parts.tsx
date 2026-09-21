// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every shared control, in each state it can be in. They are gathered
// in one component rather than spread through the index because the
// states that need a signal - a chosen model, a ticked row, an open
// dialog - are held here beside the fixtures that show them.

import { createSignal } from "solid-js";

import { useSay } from "../../ui";
import { Badge } from "../parts/badge";
import { Banner } from "../parts/banner";
import { Button } from "../parts/button";
import { Combobox } from "../parts/combobox";
import { Dialog } from "../parts/dialog";
import { EmptyState } from "../parts/empty";
import { Field } from "../parts/field";
import { Notice } from "../parts/notice";
import { Progress } from "../parts/progress";
import { Row } from "../parts/row";
import { Skeleton } from "../parts/skeleton";
import { Table, type Column } from "../parts/table";
import { Tabs } from "../parts/tabs";
import { Case } from "./case";
import { CHOSEN, MODELS, type ModelRow } from "./served";

// The four readings of one run, which is what the lens switcher is for.
const LENSES = ["run_turns", "run_context", "run_changes", "run_evidence"] as const;

export function Parts() {
  const say = useSay();
  const [host, setHost] = createSignal("api.zenmux.ai");
  const [bad, setBad] = createSignal("api_gateway.internal");
  const [key, setKey] = createSignal("secret:providers/zenmux");
  const [model, setModel] = createSignal<string | null>(null);
  const [rows, setRows] = createSignal<readonly ModelRow[]>(MODELS);
  const [picked, setPicked] = createSignal<readonly string[]>([CHOSEN.id]);
  const [asking, setAsking] = createSignal(false);
  const [lens, setLens] = createSignal<string>("run_turns");

  const columns: readonly Column<ModelRow>[] = [
    {
      key: "id",
      header: say("part_model_id"),
      render: (row) => <span class="font-mono text-note">{row.id}</span>,
      compare: (a, b) => a.id.localeCompare(b.id),
    },
    {
      key: "context",
      header: say("part_context_window"),
      render: (row) => <span class="font-mono text-note">{row.context}</span>,
      compare: (a, b) => Number(a.context) - Number(b.context),
    },
    {
      key: "ceiling",
      header: say("part_output_ceiling"),
      render: (row) => <span class="font-mono text-note">{row.ceiling}</span>,
      editable: {
        text: (row) => row.ceiling,
        onEdit: (row, text) => {
          setRows(rows().map((each) => (each.id === row.id ? { ...each, ceiling: text } : each)));
        },
      },
    },
  ];

  return (
    <>
      <Case label="button · four tones">
        <div class="flex flex-wrap items-center gap-snug">
          <Button label={say("part_save")} tone="primary" />
          <Button label={say("part_cancel")} tone="secondary" />
          <Button label={say("dismiss")} tone="quiet" />
          <Button label={say("part_delete")} tone="destructive" />
        </div>
      </Case>

      <Case label="button · loading and refused">
        <div class="flex flex-wrap items-center gap-snug">
          <Button label={say("part_save")} tone="primary" loading />
          <Button label={say("part_save")} tone="primary" why={say("part_why_halted")} />
        </div>
      </Case>

      <Case label="field · help, error, mono with fixed parts">
        <div class="flex flex-col gap-base">
          <Field
            label={say("setup_base_url")}
            help={say("part_help_base_url")}
            value={host()}
            onInput={setHost}
            prefix="https://"
            suffix="/v1"
          />
          <Field
            label={say("setup_base_url")}
            error={say("part_error_host")}
            value={bad()}
            onInput={setBad}
          />
          <Field label={say("setup_key")} value={key()} onInput={setKey} mono />
        </div>
      </Case>

      <Case label="combobox · searchable, nothing chosen">
        <Combobox
          label={say("setup_models")}
          placeholder={say("part_search")}
          empty={say("part_no_match")}
          choices={MODELS.map((each) => ({ value: each.id, label: each.id, note: each.context }))}
          value={model()}
          onPick={setModel}
        />
      </Case>

      <Case label="table · ticked, sortable, corrected in place">
        <Table
          caption={say("setup_models")}
          columns={columns}
          rows={rows()}
          keyOf={(row) => row.id}
          selection={{
            picked: (row) => picked().includes(row.id),
            onPick: (row, on) => {
              setPicked(on ? [...picked(), row.id] : picked().filter((each) => each !== row.id));
            },
            allLabel: say("part_select_all"),
            allPicked: () => picked().length === rows().length,
            onPickAll: (on) => {
              setPicked(on ? rows().map((each) => each.id) : []);
            },
          }}
        />
      </Case>

      <Case label="table · empty">
        <Table
          caption={say("setup_models")}
          columns={columns}
          rows={[]}
          keyOf={(row) => row.id}
          empty={
            <EmptyState
              text={say("setup_no_models")}
              action={<Button label={say("setup_look")} tone="primary" />}
            />
          }
        />
      </Case>

      <Case label="row · status and actions">
        <div class="rounded-card border border-edge-panel">
          <Row
            primary="hall/mayor"
            secondary={say("talk_tokens", { n: "12480" })}
            status={<Badge text={say("status_in_progress")} weight="live" dot />}
            actions={<Button label={say("talk_stop")} tone="quiet" />}
            onOpen={() => undefined}
          />
          <Row
            primary="lab/east"
            secondary={say("tree_no_runs")}
            status={<Badge text={say("city_quiet")} dot />}
          />
        </div>
      </Case>

      <Case label="banner · stopped, not connected, waiting">
        <div class="flex flex-col gap-snug">
          <Banner
            text={say("talk_halted")}
            detail={say("city_active", { n: "3" })}
            weight="alert"
            action={<Button label={say("talk_release")} tone="secondary" />}
          />
          <Banner
            text={say("talk_not_live")}
            weight="alert"
            action={<Button label={say("link_retry")} tone="secondary" />}
          />
          <Banner
            text={say("wait_title")}
            detail={say("nav_waiting", { n: "2" })}
            action={<Button label={say("wait_allow")} tone="primary" />}
          />
        </div>
      </Case>

      <Case label="notice · toast and entry in the centre">
        <div class="flex flex-col gap-snug">
          <Notice
            title={say("part_saved")}
            detail={say("setup_base_url")}
            at="03:35"
            action={<Button label={say("part_undo")} tone="quiet" />}
            dismiss={<Button label={say("dismiss")} tone="quiet" />}
          />
          <div class="rounded-card border border-edge-panel">
            <Notice seat="entry" title={say("part_saved")} at="03:35" />
            <Notice
              seat="entry"
              weight="alert"
              title={say("link_refused")}
              detail={say("part_error_host")}
              at="03:31"
            />
          </div>
        </div>
      </Case>

      <Case label="dialog · confirming what cannot be undone">
        <Button
          label={say("setup_remove")}
          tone="destructive"
          onPress={() => {
            setAsking(true);
          }}
        />
        <Dialog
          open={asking()}
          title={say("part_remove_endpoint")}
          detail={say("part_remove_detail")}
          confirmLabel={say("part_delete")}
          cancelLabel={say("part_cancel")}
          destructive
          onConfirm={() => {
            setAsking(false);
          }}
          onCancel={() => {
            setAsking(false);
          }}
        />
      </Case>

      <Case label="progress · counted and uncounted">
        <div class="flex flex-col gap-base">
          <Progress label={say("machine_ready")} done={4} total={6} />
          <Progress label={say("machine_ready")} done={0} total={0} />
        </div>
      </Case>

      <Case label="empty state">
        <EmptyState
          text={say("city_no_buildings")}
          action={<Button label={say("part_new_building")} tone="primary" />}
        />
      </Case>

      <Case label="skeleton · prose and rows">
        <div class="flex flex-col gap-wide">
          <Skeleton label={say("part_loading")} rows={3} />
          <Skeleton label={say("part_loading")} rows={4} tall />
        </div>
      </Case>

      <Case label="tabs · the lenses of one run">
        <Tabs
          label={say("run_lenses")}
          lenses={LENSES.map((key) => ({
            id: key,
            label: say(key),
            mark: key === "run_changes" ? <Badge text="7" /> : undefined,
          }))}
          current={lens()}
          onPick={setLens}
        />
      </Case>

      <Case label="badge · count and state">
        <div class="flex flex-wrap items-center gap-snug">
          <Badge text={say("nav_waiting", { n: "3" })} />
          <Badge text={say("status_in_progress")} weight="live" dot />
          <Badge text={say("status_blocked")} weight="alert" dot />
        </div>
      </Case>
    </>
  );
}
