// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";
import type { Reason } from "../setup/dependencies";
import type { InstallChannel, Registry, RegistryNewest, ReleaseLine, UpdateHint } from "../../wire";

const REGISTRY: Readonly<Record<Registry, Key>> = {
  npm: "release_registry_npm",
  crates_io: "release_registry_crates_io",
  github: "release_registry_github",
};

export type RegistryLine =
  | { readonly registry: Key; readonly newest: ReleaseLine; readonly reason: null }
  | { readonly registry: Key; readonly newest: null; readonly reason: Reason };

export function registryLineOf(line: RegistryNewest): RegistryLine {
  const registry = REGISTRY[line.registry];
  const reading = line.reading;
  return "read" in reading
    ? { registry, newest: reading.read.newest, reason: null }
    : {
        registry,
        newest: null,
        reason: { key: "release_registry_refused", said: reading.refused.refusal.recovery },
      };
}

const CHANNEL: Readonly<Record<InstallChannel, Key>> = {
  npm: "release_channel_npm",
  bun: "release_channel_bun",
  binstall: "release_channel_binstall",
  unknown: "release_channel_unknown",
  package: "release_channel_package",
  cargo_or_binstall: "release_channel_cargo_or_binstall",
  cargo: "release_channel_cargo",
  archive: "release_channel_archive",
  aur: "release_channel_aur",
  source: "release_channel_source",
};

export interface Update {
  readonly channel: Key;
  readonly command: string | null;
}

export function updateOf(hint: UpdateHint): Update {
  return { channel: CHANNEL[hint.channel], command: hint.command ?? null };
}

export function confirmedCommand(hint: UpdateHint, requested: string | null): string | null {
  return hint.alternatives.find((command) => command === requested) ?? null;
}
