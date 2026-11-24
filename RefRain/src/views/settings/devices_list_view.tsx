// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, For, Show } from "solid-js";

import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ago, clock } from "../../core/time";
import { ui } from "../../ui";
import type { DeviceId, DeviceLine } from "../../wire";
import Badge from "../parts/badge.solid";
import Button from "../parts/button.solid";
import Dialog from "../parts/dialog_view";
import EmptyState from "../parts/empty.solid";
import Row, { RowList } from "../parts/row.solid";

export interface DevicesListProps {
  readonly devices: readonly DeviceLine[];
  readonly here: string | null;
  readonly now: number;
  readonly local: boolean;
  readonly onForget: (device: DeviceId) => void;
}

export default function DeviceList(props: DevicesListProps) {
  const lang = storeValue(ui().lang);
  const word = (key: Key) => say(lang(), key);
  const [asking, setAsking] = createSignal<DeviceLine>();
  const duplicates = createMemo(() => {
    const seen = new Set<string>();
    const repeated = new Set<string>();
    for (const device of props.devices) {
      if (seen.has(device.label)) repeated.add(device.label);
      seen.add(device.label);
    }
    return repeated;
  });

  function named(device: DeviceLine): string {
    return duplicates().has(device.label)
      ? `${device.label} · ${clock(lang(), device.paired_at)}`
      : device.label;
  }

  function seen(device: DeviceLine): string {
    const paired = `${word("devices_paired_at")} ${ago(lang(), device.paired_at, props.now)}`;
    const last =
      device.last_seen == null
        ? word("devices_never_seen")
        : `${word("devices_last_seen")} ${ago(lang(), device.last_seen, props.now)}`;
    return `${paired} · ${last}`;
  }

  const label = () => {
    const current = asking();
    return current === undefined ? "" : named(current);
  };

  return (
    <>
      <div class="flex flex-col gap-base">
        <Show when={!props.local}>
          <p class="text-note text-text-quiet">{word("devices_local_only")}</p>
        </Show>
        <Show
          when={props.devices.length > 0}
          fallback={<EmptyState missing="devices_none" seat="region" />}
        >
          <div class="rounded-card border border-edge-panel">
            <RowList label={word("devices_title")}>
              <For each={props.devices} keyed={(device) => device.id}>
                {(device) => (
                  <Row
                    primary={named(device())}
                    secondary={seen(device())}
                    status={
                      <Show when={device().id === props.here}>
                        <Badge text={word("devices_this")} dot />
                      </Show>
                    }
                    actions={
                      <Show when={props.local}>
                        <Button
                          label={word("devices_forget_verb")}
                          name={fill(word("devices_forget"), { label: named(device()) })}
                          tone="quiet"
                          onPress={() => {
                            setAsking(device());
                          }}
                        />
                      </Show>
                    }
                  />
                )}
              </For>
            </RowList>
          </div>
        </Show>
      </div>
      <Dialog
        open={asking() !== undefined}
        title={fill(word("devices_forget_title"), { label: label() })}
        detail={word("devices_forget_detail")}
        confirmLabel={fill(word("devices_forget"), { label: label() })}
        cancelLabel={word("quit_cancel")}
        destructive
        onConfirm={() => {
          const chosen = asking();
          if (chosen !== undefined) props.onForget(chosen.id);
          setAsking(undefined);
        }}
        onCancel={() => {
          setAsking(undefined);
        }}
      />
    </>
  );
}
