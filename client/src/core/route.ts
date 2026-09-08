// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one translation between a View and the address bar, both ways.
// One spelling is written; every spelling an older build wrote is read.
// Ported table for table from `crates/web/src/route/fragments.rs`.

import { Option } from "effect";

import { Address } from "./address";
import { RunId } from "./run_id";

// Which lens the record is read through: the whole history, then what was
// kept, then what was thrown away.
export type Lens = "ledger" | "archive" | "bin";

export const LENSES: readonly Lens[] = ["ledger", "archive", "bin"];

// Which page the content region shows.
export type View =
  | { readonly kind: "sessions" }
  | { readonly kind: "session"; readonly address: Address }
  | { readonly kind: "waiting" }
  | { readonly kind: "record"; readonly lens: Lens }
  | { readonly kind: "cost" }
  | { readonly kind: "setup" }
  | { readonly kind: "building"; readonly address: Address }
  | { readonly kind: "run"; readonly run: RunId };

// The list, and the box that starts work: the page a person arrives to.
export const DEFAULT_VIEW: View = { kind: "sessions" };

// The address-bar form of a view, fragment marker included. Always begins
// `#/`, so a fragment written by hand and one written here are the same
// string. Each view has exactly one spelling.
export function toFragment(view: View): string {
  switch (view.kind) {
    case "sessions":
      return "#/";
    case "session":
      return `#/s/${view.address}`;
    case "waiting":
      return "#/waiting";
    case "record":
      return recordFragment(view.lens);
    case "cost":
      return "#/cost";
    case "setup":
      return "#/setup";
    case "building":
      return `#/b/${view.address}`;
    case "run":
      return `#/live/${view.run}`;
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
// spelling this build no longer writes: each lands on the page that
// inherited its question, and the three that had a page of their own
// became one lens each of the record.
const BARE: Readonly<Record<string, View>> = {
  "": DEFAULT_VIEW,
  waiting: { kind: "waiting" },
  record: { kind: "record", lens: "ledger" },
  cost: { kind: "cost" },
  setup: { kind: "setup" },

  overview: DEFAULT_VIEW,
  city: DEFAULT_VIEW,
  live: DEFAULT_VIEW,
  approvals: { kind: "waiting" },
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
    case "s":
      return Option.map(Address.option(tail), (address) => ({
        kind: "session",
        address,
      }));
    case "b":
    case "building":
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
