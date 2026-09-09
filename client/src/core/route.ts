// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one translation between a View and the address bar, both ways.
// One spelling is written; every spelling an older build wrote is read.
// The old table (`crates/web/src/route/fragments.rs`) is folded in at
// the bottom, so a bookmark from the Dioxus client still opens.

import { Option } from "effect";

import { Address } from "./address";
import { RunId } from "./run_id";

// Which lens the record is read through: the whole history, then what was
// kept, then what was thrown away.
export type Lens = "ledger" | "archive" | "bin";

export const LENSES: readonly Lens[] = ["ledger", "archive", "bin"];

// The room a person talks to when they have named none: the Mayor.
export const MAYOR: Address = Address("hall/mayor");

// Which page the content region shows.
export type View =
  | { readonly kind: "talk"; readonly address: Address }
  | { readonly kind: "city" }
  | { readonly kind: "building"; readonly address: Address }
  | { readonly kind: "run"; readonly run: RunId }
  | { readonly kind: "setup" }
  | { readonly kind: "record"; readonly lens: Lens }
  | { readonly kind: "cost" }
  | { readonly kind: "welcome" };

// The conversation with the Mayor: the page a person arrives to.
export const DEFAULT_VIEW: View = { kind: "talk", address: MAYOR };

// The address-bar form of a view, fragment marker included. Always begins
// `#/`, so a fragment written by hand and one written here are the same
// string. Each view has exactly one spelling.
export function toFragment(view: View): string {
  switch (view.kind) {
    case "talk":
      return view.address === MAYOR ? "#/" : `#/talk/${view.address}`;
    case "city":
      return "#/city";
    case "building":
      return `#/building/${view.address}`;
    case "run":
      return `#/run/${view.run}`;
    case "setup":
      return "#/setup";
    case "record":
      return recordFragment(view.lens);
    case "cost":
      return "#/cost";
    case "welcome":
      return "#/welcome";
  }
}

function recordFragment(lens: Lens): string {
  switch (lens) {
    case "ledger":
      return "#/record";
    case "archive":
      return "#/record/archive";
    case "bin":
      return "#/record/bin";
  }
}

// A head with nothing after it. The lower half of the table is every
// spelling an older build wrote: each lands on the page that inherited
// its question.
const BARE: Readonly<Record<string, View>> = {
  "": DEFAULT_VIEW,
  talk: DEFAULT_VIEW,
  city: { kind: "city" },
  setup: { kind: "setup" },
  record: { kind: "record", lens: "ledger" },
  cost: { kind: "cost" },
  welcome: { kind: "welcome" },

  overview: { kind: "city" },
  live: DEFAULT_VIEW,
  sessions: DEFAULT_VIEW,
  waiting: DEFAULT_VIEW,
  approvals: DEFAULT_VIEW,
  ledger: { kind: "record", lens: "ledger" },
  archive: { kind: "record", lens: "archive" },
  "recycle-bin": { kind: "record", lens: "bin" },
  dashboard: { kind: "cost" },
  settings: { kind: "setup" },
};

function named(raw: string): string {
  return raw.replace(/^#*/, "").replace(/^\/*/, "");
}

// The view a fragment names, or `None` when it names nothing. `None`
// rather than a silent fall back to the first page: a router that quietly
// lands somewhere else teaches people their bookmarks are unreliable
// without ever admitting it.
export function fromFragment(raw: string): Option.Option<View> {
  const path = named(raw);
  const slash = path.indexOf("/");
  const head = slash < 0 ? path : path.slice(0, slash);
  const tail = slash < 0 ? "" : path.slice(slash + 1);
  if (tail === "") {
    return Option.fromNullable(BARE[head]);
  }
  switch (head) {
    case "talk":
    case "s":
      return Option.map(Address.option(tail), (address) => ({
        kind: "talk",
        address,
      }));
    case "building":
    case "b":
      return Option.map(Address.option(tail), (address) => ({
        kind: "building",
        address,
      }));
    case "record":
      return tail === "archive"
        ? Option.some({ kind: "record", lens: "archive" })
        : tail === "bin"
          ? Option.some({ kind: "record", lens: "bin" })
          : Option.none();
    case "run":
    case "live":
      return Option.map(RunId.option(tail), (run) => ({ kind: "run", run }));
    default:
      return Option.none();
  }
}

// What the address bar says, when this build cannot resolve it. `None`
// for an empty fragment, which is the first page rather than a broken
// link.
export function unresolved(hash: string): Option.Option<string> {
  const name = named(hash);
  if (name === "" || Option.isSome(fromFragment(hash))) {
    return Option.none();
  }
  return Option.some(`#/${name}`);
}

// The address bar, as the one thing this module reads and writes on it.
// `window.location` is one; a test hands in a plain object.
export interface AddressBar {
  hash: string;
}

export function current(bar: Readonly<AddressBar>): Option.Option<View> {
  return fromFragment(bar.hash);
}

// Puts a view in the address bar. Assigning the hash pushes a history
// entry and fires `hashchange`, and that listener is what moves the
// signal: a click, an `<a href>` and the back button take one path.
export function go(bar: AddressBar, view: View): void {
  bar.hash = toFragment(view);
}

// The building an address belongs to: its first segment.
export function buildingOf(address: Address): Address {
  const slash = address.indexOf("/");
  return slash < 0 ? address : Address(address.slice(0, slash));
}

// The last segment: what a person called the room.
export function roomOf(address: Address): string {
  const slash = address.lastIndexOf("/");
  return slash < 0 ? address : address.slice(slash + 1);
}
