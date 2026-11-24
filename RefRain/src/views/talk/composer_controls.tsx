// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import type { Sending } from "../../core/doing";
import { canRecord } from "../../core/speaking";
import type { Address, RunId } from "../../wire";
import type { PopoverBinding, PopoverColumn, PopoverRow } from "../parts/popover";
import Popover from "../parts/popover_view";
import Unkept from "../parts/unkept_view";
import type { Face } from "./coin_face";
import Coin from "./coin_view";
import { Handed } from "./delivery_view";
import DropRefused from "./drop_refused_view";
import type { Refused } from "./dropping";
import Gauge from "./gauge_view";
import type { PickerFacts } from "./picker_scene";
import Record from "./record_view";
import SettingsRow from "./settings_row_view";
import type { RowDraws } from "./settings_row";
import type { SpeechState } from "./speech";

export function ComposerControls(props: {
  readonly room: Address | null;
  readonly run: RunId | undefined;
  readonly historical: boolean;
  readonly face: Face;
  readonly sending: Sending;
  readonly delivery: SpeechState["kind"];
  readonly onStop: () => void;
  readonly onWords?: ((words: string) => void) | undefined;
}) {
  return (
    <>
      <Show when={canRecord() ? props.onWords : undefined}>
        {(write) => <Record onWords={(words) => write()(words)} />}
      </Show>
      <Gauge room={props.room} run={props.run} historical={props.historical}>
        <Coin
          face={props.face}
          sending={props.sending}
          delivery={props.delivery}
          onStop={props.onStop}
        />
      </Gauge>
    </>
  );
}

export function ComposerCommands(props: {
  readonly open: boolean;
  readonly columns: readonly PopoverColumn[];
  readonly onApply: (column: PopoverColumn, row: PopoverRow) => void;
  readonly onClose: () => void;
  readonly bind: (binding: PopoverBinding) => void;
  readonly onCursor: (id: string | null) => void;
  readonly onRow: (id: string | undefined) => void;
}) {
  return (
    <Show when={props.open && props.columns.length > 0}>
      <Popover
        label="talk_commands"
        columns={props.columns}
        onApply={props.onApply}
        onClose={props.onClose}
        bind={props.bind}
        onCursorChange={props.onCursor}
        onCursorRow={(row) => props.onRow(row?.id)}
      />
    </Show>
  );
}

export function ComposerFooter(props: {
  readonly room: Address | null;
  readonly sent: number;
  readonly words: () => string;
  readonly unkept: boolean;
  readonly refused: readonly Refused[];
  readonly picker: PickerFacts;
  readonly draws: RowDraws;
  readonly kept: boolean;
}) {
  return (
    <>
      <Handed room={props.room} sent={props.sent} />
      <Show when={props.unkept && props.words() !== ""}>
        <Unkept words={props.words} />
      </Show>
      <DropRefused refused={props.refused} />
      <SettingsRow picker={props.picker} room={props.room} draws={props.draws} kept={props.kept} />
    </>
  );
}
