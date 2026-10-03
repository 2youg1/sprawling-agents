// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the release group reads out of the city's answer
// (`crates/wire/spec/Answer/Release.lean` D24): one line per registry the
// city asked, and the command that updates this binary through the
// channel that installed it.
//
// The verdict stays the city's: it judges npm's line, and the page does
// not compare the crates.io version against this one, because the city
// has no rule yet for how the two registries spell a pre-release.

import type { Key } from "../../core/lang";
import type { Reason } from "../setup/dependencies";
import type { InstallChannel, Registry, RegistryNewest, ReleaseLine, UpdateHint } from "../../wire";

const REGISTRY: Readonly<Record<Registry, Key>> = {
  npm: "release_registry_npm",
  crates_io: "release_registry_crates_io",
};

// The newest release a registry read, or why it has none: exactly one
// of the two is set.
export type RegistryLine =
  | { readonly registry: Key; readonly newest: ReleaseLine; readonly reason: null }
  | { readonly registry: Key; readonly newest: null; readonly reason: Reason };

export function registryLineOf(line: RegistryNewest): RegistryLine {
  const registry = REGISTRY[line.registry];
  const reading = line.reading;
  return "read" in reading
    ? { registry, newest: reading.read.newest, reason: null }
    : { registry, newest: null, reason: { key: "release_registry_refused", said: reading.refused.refusal.recovery } };
}

// npm's newest release, which the verdict judges.
export function npmNewest(registries: readonly RegistryNewest[]): ReleaseLine | null {
  for (const line of registries) {
    if (line.registry === "npm" && "read" in line.reading) {
      return line.reading.read.newest;
    }
  }
  return null;
}

const CHANNEL: Readonly<Record<InstallChannel, Key>> = {
  npm: "release_channel_npm",
  cargo: "release_channel_cargo",
  archive: "release_channel_archive",
  source: "release_channel_source",
};

// How this binary was installed, and the command the city printed for
// updating it; a binary built from source has no command, and the page
// says to pull and rebuild instead.
export interface Update {
  readonly channel: Key;
  readonly command: string | null;
}

export function updateOf(hint: UpdateHint): Update {
  return { channel: CHANNEL[hint.channel], command: hint.command ?? null };
}
