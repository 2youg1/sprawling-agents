// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { wireApiOf } from "../../../core/commands";
import type { WireApi } from "../../../core/commands";
import type { KnownHost } from "../../../wire";
import { hostOf } from "./draft";

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

export function rowFor(hosts: readonly KnownHost[], baseUrl: string): KnownHost | null {
  const host = hostOf(baseUrl)?.toLowerCase();
  if (host === undefined) return null;
  return hosts.find((row) => row.host === host) ?? null;
}

export function facesOf(row: KnownHost): readonly WireApi[] {
  return row.faces.map((face) => wireApiOf(face.dialect));
}

export function picked(
  row: KnownHost,
  wireApi: WireApi,
): { readonly wireApi: WireApi; readonly baseUrl: string } | null {
  const face = row.faces.find((each) => wireApiOf(each.dialect) === wireApi) ?? row.faces[0];
  return face === undefined ? null : { wireApi: wireApiOf(face.dialect), baseUrl: face.base_url };
}

export function followed(row: KnownHost, baseUrl: string, wireApi: WireApi): string | null {
  const typed = baseUrl.trim();
  if (!row.faces.some((face) => face.base_url === typed)) return null;
  const face = row.faces.find((each) => wireApiOf(each.dialect) === wireApi);
  return face === undefined || face.base_url === typed ? null : face.base_url;
}
