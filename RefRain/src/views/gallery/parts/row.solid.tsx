// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../../core/lang";
import { ui } from "../../../ui";
import Badge from "../../parts/badge.solid";
import Button from "../../parts/button.solid";
import EmptyState from "../../parts/empty.solid";
import Progress from "../../parts/progress.solid";
import Row, { RowList } from "../../parts/row.solid";
import Skeleton from "../../parts/skeleton.solid";
import { GalleryCase } from "../case";
import { storeValue } from "../../../core/solid_store";

export default function RowCases() {
  const lang = storeValue(ui().lang);

  function walkedRows() {
    return (
      <>
        <Row
          primary="openai/gpt-nucleus-6"
          secondary={fill(say(lang(), "talk_tokens"), { n: "12480" })}
          onOpen={() => undefined}
          status={<Badge text={say(lang(), "status_in_progress")} weight="live" dot />}
          actions={<Button label={say(lang(), "mcp_check")} tone="quiet" />}
        />
        <Row
          primary="anthropic/claude-lattice"
          secondary={say(lang(), "tree_no_runs")}
          onOpen={() => undefined}
          chosen
        />
        <Row
          primary="example/local-mirror"
          secondary={say(lang(), "tree_no_runs")}
          status={<Badge text={say(lang(), "city_quiet")} dot />}
        />
      </>
    );
  }

  return (
    <>
      <GalleryCase label="row · a walked list, the second row chosen">
        <div class="rounded-card border border-edge-panel">
          <RowList label={say(lang(), "mcp_servers")}>{walkedRows()}</RowList>
        </div>
      </GalleryCase>

      <GalleryCase label="progress · part of the way, at its end, and with no end yet">
        <div class="flex flex-col gap-base">
          <Progress label={say(lang(), "machine_walk_progress")} done={3} total={12} />
          <Progress label={say(lang(), "machine_walk_progress")} done={12} total={12} />
          <Progress label={say(lang(), "machine_progress_label")} done={0} total={0} />
        </div>
      </GalleryCase>

      <GalleryCase label="progress · with no end yet, motion off">
        <div data-motion="off">
          <Progress label={say(lang(), "machine_progress_label")} done={0} total={0} />
        </div>
      </GalleryCase>

      <GalleryCase label="skeleton · a paragraph, then a list of rows">
        <div class="flex flex-col gap-wide">
          <Skeleton label={say(lang(), "machine_checking")} rows={4} />
          <Skeleton label={say(lang(), "machine_checking")} rows={3} tall />
        </div>
      </GalleryCase>

      <GalleryCase label="empty · centred, with its way out">
        <EmptyState
          missing="rec_nothing"
          action={<Button label={say(lang(), "city_ask_mayor")} tone="primary" />}
        />
      </GalleryCase>

      <GalleryCase label="empty · in the region a list would take">
        <EmptyState missing="mcp_none" seat="region" />
      </GalleryCase>

      <GalleryCase label="empty · inset in a frame that draws the region">
        <div class="rounded-card border border-edge-panel">
          <EmptyState missing="rec_nothing" seat="inset" />
        </div>
      </GalleryCase>
    </>
  );
}
