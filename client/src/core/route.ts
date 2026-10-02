// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one translation between a View and the address bar, both ways.
// One spelling is written; every spelling an older build wrote is read.
// The old table (`crates/web/src/route/fragments.rs`) is folded in at
// the bottom, so a bookmark from the Dioxus client still opens.

import { Option, Schema } from "effect";

import { Address, B3Hash, GuideStep, Seq, SessionName } from "../wire";
import type { RunId } from "../wire";
import { isInvitation } from "./remote/invitation";
import { readRunId } from "./run_id";

// The address grammar is the server's, carried in the schema `cargo
// xtask wire-ts` generates from `kernel::Address`. A fragment a person
// edited by hand is read through it and nowhere else.
const readAddress = Schema.decodeOption(Address);

// Which lens the record is read through: the whole history, then what was
// kept, then what was thrown away.
export type Lens = "ledger" | "archive" | "bin" | "log";

export const LENSES: readonly Lens[] = ["ledger", "archive", "bin", "log"];

// The groups the settings panel draws in its body, as the address bar
// spells them: `#/setup/<group>` opens the panel at one (client/Spec.lean §7L).
// The order the settings tree offers them in is the tree's own
// (`views/settings/tree.ts`); this is only the set the address bar reads.
export const SETUP_GROUPS = [
  "you", "accounts", "harnesses", "network", "remote", "run", "rules", "automation",
  "skills", "tools", "appearance", "keys", "advanced", "about",
] as const;

export type SetupGroup = (typeof SETUP_GROUPS)[number];

// The lenses of the run page, in the order its tabs offer them; a
// `#/run/<id>/<lens>` link opens the page at one (client D28).
export const RUN_LENSES = ["time", "turns", "monitor", "prompt", "context", "changes", "evidence"] as const;

export type RunLens = (typeof RUN_LENSES)[number];

// An item of the right side, as a link names it (client/Spec.lean §4-63): a
// call, or a version of a building's document - the worktree's current
// text when `version` is `null`. The same two shapes the inspector holds
// (`views/inspect/open.svelte.ts`), so opening one hands it over whole.
export type ItemLink =
  | { readonly kind: "call"; readonly run: RunId; readonly at: Seq }
  | { readonly kind: "document"; readonly building: Address; readonly path: string; readonly version: B3Hash | null };

// The room a person talks to when they have named none: the Mayor.
export const MAYOR: Address = Address.make("hall/mayor");

// Which page the content region shows.
export type View =
  // A conversation, and the item a link asks the right side to open
  // beside it on arrival; the right side's state itself is not here
  // (client/Spec.lean §4-27).
  | { readonly kind: "talk"; readonly address: Address; readonly item?: ItemLink }
  | { readonly kind: "city" }
  | { readonly kind: "building"; readonly address: Address }
  // A run, and the lens a link opened it at; without one the page picks
  // its lens from the run's state.
  | { readonly kind: "run"; readonly run: RunId; readonly lens?: RunLens }
  // The settings panel, open over the page beneath it. Without a group
  // the panel opens at the one it drew last: the address bar names a
  // group only when somebody chose one.
  | { readonly kind: "setup"; readonly group?: SetupGroup }
  | { readonly kind: "mcp" }
  | { readonly kind: "record"; readonly lens: Lens }
  | { readonly kind: "cost" }
  | { readonly kind: "registry" }
  // The guide, and the step a link opened; without one the guide opens
  // at the step it judges current.
  | { readonly kind: "welcome"; readonly step?: GuideStep }
  | { readonly kind: "monitor" }
  // Every screen at once, on fixtures, reachable without a city. It is
  // a route rather than a build flag so the gate that measures it opens
  // the same bundle a person runs.
  | { readonly kind: "gallery" };

// The conversation with the Mayor: the page a person arrives to.
export const DEFAULT_VIEW: View = { kind: "talk", address: MAYOR };

// The address-bar form of a view, fragment marker included. Always begins
// `#/`, so a fragment written by hand and one written here are the same
// string. Each view has exactly one spelling.
export function toFragment(view: View): string {
  switch (view.kind) {
    case "talk":
      if (view.item !== undefined) return `#/talk/${escaped(view.address)}?${itemQuery(view.item)}`;
      return view.address === MAYOR ? "#/" : `#/talk/${escaped(view.address)}`;
    case "city":
      return "#/city";
    case "building":
      return `#/building/${escaped(view.address)}`;
    case "run":
      return view.lens === undefined ? `#/run/${view.run}` : `#/run/${view.run}/${view.lens}`;
    case "setup":
      return view.group === undefined ? "#/setup" : `#/setup/${view.group}`;
    case "mcp":
      return "#/mcp";
    case "record":
      return recordFragment(view.lens);
    case "registry":
      return "#/registry";
    case "cost":
      return "#/cost";
    case "welcome":
      return view.step === undefined ? "#/welcome" : `#/welcome/${view.step}`;
    case "monitor":
      return "#/monitor";
    case "gallery":
      return "#/gallery";
  }
}

// An item as the query after a conversation's address: every value
// percent-encoded by the browser's own writer, a worktree's text with no
// `version` key at all.
function itemQuery(item: ItemLink): string {
  switch (item.kind) {
    case "call":
      return new URLSearchParams({ call: item.run, at: String(item.at) }).toString();
    case "document":
      return new URLSearchParams({
        building: item.building,
        path: item.path,
        ...(item.version === null ? {} : { version: item.version }),
      }).toString();
  }
}

// The item a conversation's query names: exactly the keys one of the two
// shapes writes, each read through the wire's own schema, or `None`.
function readItem(query: string): Option.Option<ItemLink> {
  const asked = new URLSearchParams(query);
  const keys = [...asked.keys()].sort().join(" ");
  const value = (key: string): string => asked.get(key) ?? "";
  switch (keys) {
    case "at call":
      return Option.map(
        Option.all([readRunId(value("call")), readSeq(value("at"))]),
        ([run, at]): ItemLink => ({ kind: "call", run, at }),
      );
    case "building path":
    case "building path version": {
      const path = value("path");
      const version: Option.Option<B3Hash | null> = keys.endsWith("version") ? readB3(value("version")) : Option.some(null);
      if (path === "") return Option.none();
      return Option.map(
        Option.all([readAddress(value("building")), version]),
        ([building, held]): ItemLink => ({ kind: "document", building, path, version: held }),
      );
    }
    default:
      return Option.none();
  }
}

const readB3 = Schema.decodeOption(B3Hash);
const readSeq = (raw: string): Option.Option<Seq> => (/^\d+$/.test(raw) ? Schema.decodeOption(Seq)(Number(raw)) : Option.none());
const readLens = (raw: string): Option.Option<RunLens> => Option.fromNullishOr(RUN_LENSES.find((lens) => lens === raw));
const readStep = (raw: string): Option.Option<GuideStep> => Option.fromNullishOr(GuideStep.literals.find((step) => step === raw));

function recordFragment(lens: Lens): string {
  switch (lens) {
    case "ledger":
      return "#/record";
    case "archive":
      return "#/record/archive";
    case "bin":
      return "#/record/bin";
    case "log":
      return "#/record/log";
  }
}

// A head with nothing after it, in the spelling this build writes.
const BARE: Readonly<Record<string, View>> = {
  "": DEFAULT_VIEW,
  talk: DEFAULT_VIEW,
  city: { kind: "city" },
  setup: { kind: "setup" },
  mcp: { kind: "mcp" },
  record: { kind: "record", lens: "ledger" },
  cost: { kind: "cost" },
  registry: { kind: "registry" },
  welcome: { kind: "welcome" },
  monitor: { kind: "monitor" },
  gallery: { kind: "gallery" },
};

// Every spelling an older build wrote: each lands on the page that
// inherited its question. Read, never written, and never offered.
const OLD: Readonly<Record<string, View>> = {
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

// The pages a person may name, for the menus that offer them: every
// head this build writes, less the empty fragment, which the address
// bar writes and nobody types. Derived from the table above, so a page
// somebody can type is a page the address bar can write back.
export const PAGES: readonly string[] = Object.keys(BARE).filter((head) => head !== "");

// The page one bare name reaches, `None` when this build has no page
// by that name. A menu that offers a page resolves it through here
// rather than spelling a fragment of its own.
export function page(name: string): Option.Option<View> {
  return Option.fromNullishOr(BARE[name] ?? OLD[name]);
}

// An address as the address bar carries it: each segment percent-encoded,
// the slashes between them kept, so a room named in any script is written
// the way a browser would write it and read back through `unescaped`.
function escaped(address: Address): string {
  return address.split("/").map(encodeURIComponent).join("/");
}

// The segments a browser percent-encoded, decoded one by one so an encoded
// slash stays inside its segment. `None` for a malformed escape, which
// names nothing rather than a room spelled with a stray `%`.
const decodeSegment = Option.liftThrowable(decodeURIComponent);

function unescaped(tail: string): Option.Option<string> {
  return Option.map(Option.all(tail.split("/").map(decodeSegment)), (segments) =>
    segments.join("/"),
  );
}

function readEscapedAddress(tail: string): Option.Option<Address> {
  return Option.flatMap(unescaped(tail), readAddress);
}

function named(raw: string): string {
  return raw.replace(/^#*/, "").replace(/^\/*/, "");
}

// The view a fragment names, or `None` when it names nothing. `None`
// rather than a silent fall back to the first page: a router that quietly
// lands somewhere else teaches people their bookmarks are unreliable
// without ever admitting it.
export function fromFragment(raw: string): Option.Option<View> {
  // The link `/remote pair` prints carries its invitation in a fragment
  // of its own shape; it opens the group that pairs (client/Spec.lean §3-2).
  if (isInvitation(raw)) return Option.some({ kind: "setup", group: "remote" });
  const whole = named(raw);
  const mark = whole.indexOf("?");
  if (mark >= 0) return talkWith(whole.slice(0, mark), whole.slice(mark + 1));
  const path = whole;
  const slash = path.indexOf("/");
  const head = slash < 0 ? path : path.slice(0, slash);
  const tail = slash < 0 ? "" : path.slice(slash + 1);
  if (tail === "") {
    return page(head);
  }
  switch (head) {
    case "talk":
    case "s":
      return Option.map(readEscapedAddress(tail), (address) => ({
        kind: "talk",
        address,
      }));
    case "building":
    case "b":
      return Option.map(readEscapedAddress(tail), (address) => ({
        kind: "building",
        address,
      }));
    case "setup":
    case "settings": {
      const group = SETUP_GROUPS.find((named) => named === tail);
      return group === undefined ? Option.none() : Option.some({ kind: "setup", group });
    }
    case "record": {
      const lens = LENSES.find((named) => named === tail && named !== "ledger");
      return lens === undefined ? Option.none() : Option.some({ kind: "record", lens });
    }
    case "run":
    case "live": {
      const cut = tail.indexOf("/");
      if (cut < 0) return Option.map(readRunId(tail), (run) => ({ kind: "run", run }));
      return Option.map(Option.all([readRunId(tail.slice(0, cut)), readLens(tail.slice(cut + 1))]), ([run, lens]) => ({ kind: "run", run, lens }));
    }
    case "welcome":
      return Option.map(readStep(tail), (step) => ({ kind: "welcome", step }));
    default:
      return Option.none();
  }
}

// A conversation with an item to open beside it: the only view a query
// follows. The address is escaped, so its own question marks are `%3F`
// and the first bare one starts the query.
function talkWith(path: string, query: string): Option.Option<View> {
  const head = ["talk/", "s/"].find((prefix) => path.startsWith(prefix));
  if (head === undefined) return Option.none();
  return Option.map(Option.all([readEscapedAddress(path.slice(head.length)), readItem(query)]), ([address, item]) => ({
    kind: "talk",
    address,
    item,
  }));
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
  return slash < 0 ? address : Address.make(address.slice(0, slash));
}

// The last segment: what a person called the room.
export function roomOf(address: Address): string {
  const slash = address.lastIndexOf("/");
  return slash < 0 ? address : address.slice(slash + 1);
}

// The room a person names inside a building: `<building>/<name>`, which
// a dispatch opens by itself, so naming a room needs no digest model to
// name it. None when the name is one the city would refuse as a session.
export function roomIn(building: Address, name: string): Option.Option<Address> {
  const named = name.trim();
  return Schema.is(SessionName)(named) ? Option.some(Address.make(`${building}/${named}`)) : Option.none();
}
