// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export const WIRE_V = 82 as const;
export const WIRE_HASH = "0cd623cf7723a33c6da58ef8b20067537171ff33452a13c52f8825c8962ed136" as const;
export const CITY_RUN = "00000000-0000-0000-0000-000000000000" as const;
export const BODY_PX = { min: 12 } as const;
export const LINE_HEIGHT = { min: 1200 } as const;
export const TRACKING = { min: -50 } as const;
export const DEFAULT_EFFORT = "high" as const;

export const SLASH = [
  { spelling: "/help", takes: "", offered: "both", says: "list these verbs" },
  { spelling: "/room", takes: "<addr>", offered: "both", says: "choose the room plain lines go to" },
  { spelling: "/new", takes: "", offered: "both", says: "start a new session in this room" },
  { spelling: "/stop", takes: "", offered: "both", says: "stop the run working in this room" },
  { spelling: "/halt", takes: "[--all]", offered: "both", says: "hold this room's building, or with --all the whole city" },
  { spelling: "/release", takes: "[--all]", offered: "both", says: "let this room's building work again, or with --all the whole city" },
  { spelling: "/model", takes: "[<id>]", offered: "both", says: "show the model, or choose one the city's endpoints offer" },
  { spelling: "/effort", takes: "[<level>]", offered: "both", says: "show the thinking level, or choose one the model offers" },
  { spelling: "/approve", takes: "[<id>]", offered: "cli", says: "approve the waiting request, or the one with this id" },
  { spelling: "/deny", takes: "[<id>]", offered: "cli", says: "deny the waiting request, or the one with this id" },
  { spelling: "/web", takes: "", offered: "cli", says: "open the WebUI and quiet this terminal; Esc comes back" },
  { spelling: "/quit", takes: "", offered: "both", says: "close the city" },
  { spelling: "/serving", takes: "", offered: "cli", says: "where this city listens, and what is running in it" },
  { spelling: "/remote", takes: "<verb>", offered: "cli", says: "open|pair|close|devices|revoke: the door a device reaches the city through" },
  { spelling: "/acp", takes: "[<text>]", offered: "both", says: "find an ACP agent, or read a pasted launch line" },
  { spelling: "/wire", takes: "<verb> [<json>]", offered: "cli", says: "send any wire command or question, its body as JSON" },
] as const;

export * from "./c00";
export type * from "./c00";
export * from "./c01";
export type * from "./c01";
export * from "./c02";
export type * from "./c02";
export * from "./c03";
export type * from "./c03";
export * from "./c04";
export type * from "./c04";
export * from "./c05";
export type * from "./c05";
export * from "./c06";
export type * from "./c06";
export * from "./c07";
export type * from "./c07";
export * from "./c08";
export type * from "./c08";
export * from "./c09";
export type * from "./c09";
export * from "./c10";
export type * from "./c10";
export * from "./c11";
export type * from "./c11";
