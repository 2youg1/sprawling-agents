// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { DoctorItem, DoctorNewest } from "../../wire";
import Badge from "../parts/badge.solid";
import { versionOf } from "../setup/dependencies";
import { behind, newestOf } from "../setup/versions";

export default function Versions(props: {
  readonly item: DoctorItem;
  readonly newest: DoctorNewest | undefined;
}) {
  const lang = storeValue(ui().lang);
  const installed = () => versionOf(props.item.state);
  const pinned = () => props.item.pinned ?? null;
  const upstream = createMemo(() => newestOf(props.newest));
  const stale = () => {
    const value = upstream();
    return "version" in value && behind(installed(), value.version);
  };
  const reading = createMemo(() => {
    const value = upstream();
    return "version" in value
      ? { version: value.version, phrase: null, said: null }
      : { version: null, phrase: say(lang(), value.key), said: value.said };
  });
  return (
    <p class="flex min-w-0 flex-wrap items-baseline gap-x-base gap-y-tight text-note">
      <Show when={installed()}>
        {(version) => (
          <span class="flex min-w-0 max-w-full items-baseline gap-tight">
            <span class="shrink-0 text-text-faint">{say(lang(), "machine_version_installed")}</span>
            <span class="min-w-0 truncate font-mono text-text">{version()}</span>
          </span>
        )}
      </Show>
      <Show when={pinned()}>
        {(version) => (
          <span class="flex items-baseline gap-tight">
            <span class="text-text-faint">{say(lang(), "machine_version_pinned")}</span>
            <span class="font-mono text-text">{version()}</span>
          </span>
        )}
      </Show>
      <Show
        when={reading().version}
        fallback={<span class="min-w-0 truncate text-text-faint">{reading().phrase}</span>}
      >
        {(version) => (
          <span class="flex items-baseline gap-tight">
            <span class="text-text-faint">{say(lang(), "machine_version_newest")}</span>
            <span class="font-mono text-text">{version()}</span>
          </span>
        )}
      </Show>
      <Show when={stale()}>
        <Badge text={say(lang(), "machine_version_behind")} weight="live" dot />
      </Show>
      <Show when={reading().said}>
        {(said) => (
          <span class="min-w-0 basis-full truncate font-mono text-text-faint">{said()}</span>
        )}
      </Show>
    </p>
  );
}
