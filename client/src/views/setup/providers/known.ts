// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The vendors the city knows by host, as the attach form reads them.
//
// **The table is the city's** (`Query::KnownHosts`, channels-SPEC
// 8-51, read from `gateway::provider::preset`). This module holds no
// host of its own: it only reads the answer - which host a URL names,
// which faces that host speaks, and which base URL a face is called
// at - so the form cannot offer a face or an address the city would
// not store.
//
// The entries are addresses, not words: no language translates them.

import { wireApiOf } from "../../../core/commands";
import type { WireApi } from "../../../core/commands";
import type { KnownHost } from "../../../wire";
import { hostOf } from "./draft";

// One row of the vendor picker: the host, and the faces it speaks as
// its second line.
export interface HostChoice {
  readonly value: string;
  readonly label: string;
  readonly note: string;
}

export function hostChoices(hosts: readonly KnownHost[]): readonly HostChoice[] {
  return hosts.map((row) => ({
    value: row.host,
    label: row.host,
    note: facesOf(row).join(" / "),
  }));
}

// The row for the host a URL names, or nothing when the city does not
// know that host. DNS names answer the same in either case.
export function rowFor(hosts: readonly KnownHost[], baseUrl: string): KnownHost | null {
  const host = hostOf(baseUrl)?.toLowerCase();
  if (host === undefined) return null;
  return hosts.find((row) => row.host === host) ?? null;
}

// The faces a known host speaks, main face first.
export function facesOf(row: KnownHost): readonly WireApi[] {
  return row.faces.map((face) => wireApiOf(face.dialect));
}

// What picking a host fills in: the face already chosen when the host
// speaks it, otherwise the host's main face, and that face's base URL.
export function picked(
  row: KnownHost,
  wireApi: WireApi,
): { readonly wireApi: WireApi; readonly baseUrl: string } | null {
  const face = row.faces.find((each) => wireApiOf(each.dialect) === wireApi) ?? row.faces[0];
  return face === undefined ? null : { wireApi: wireApiOf(face.dialect), baseUrl: face.base_url };
}

// The base URL a changed face moves the box to, when the box still
// holds an address the picker filled in. An address the person typed
// stays as typed: the face control is not a second way to edit it.
export function followed(row: KnownHost, baseUrl: string, wireApi: WireApi): string | null {
  const typed = baseUrl.trim();
  if (!row.faces.some((face) => face.base_url === typed)) return null;
  const face = row.faces.find((each) => wireApiOf(each.dialect) === wireApi);
  return face === undefined || face.base_url === typed ? null : face.base_url;
}
