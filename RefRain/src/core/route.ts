// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import { Address, B3Hash, GuideStep, Seq, SessionName } from "../wire";
import type { BuildingProgress, RunId } from "../wire";
import { isInvitation } from "./remote/invitation";
import { readRunId } from "./run_id";

const readAddress = Schema.decodeOption(Address);

export type Lens = "ledger" | "archive" | "bin" | "log";

export const LENSES: readonly Lens[] = ["ledger", "archive", "bin", "log"];

export const SETUP_GROUPS = [
  "you",
  "accounts",
  "agents",
  "network",
  "remote",
  "devices",
  "run",
  "rules",
  "automation",
  "skills",
  "tools",
  "performance",
  "appearance",
  "colours",
  "keys",
  "advanced",
  "privacy",
  "about",
] as const;

export type SetupGroup = (typeof SETUP_GROUPS)[number];

export const RUN_LENSES = [
  "time",
  "turns",
  "monitor",
  "prompt",
  "context",
  "changes",
  "evidence",
] as const;

export type RunLens = (typeof RUN_LENSES)[number];

export type ItemLink =
  | { readonly kind: "call"; readonly run: RunId; readonly at: Seq }
  | {
      readonly kind: "document";
      readonly building: Address;
      readonly path: string;
      readonly version: B3Hash | null;
    };

export const MAYOR: Address = Address.make("hall/mayor");

const STRETCH = ":";

export type View =
  | {
      readonly kind: "talk";
      readonly address: Address;
      readonly item?: ItemLink;
      readonly session?: Seq;
    }
  | { readonly kind: "city" }
  | { readonly kind: "building"; readonly address: Address; readonly section?: "catalog" }
  | { readonly kind: "raise"; readonly address?: Address }
  | { readonly kind: "run"; readonly run: RunId; readonly lens?: RunLens }
  | { readonly kind: "setup"; readonly group?: SetupGroup }
  | { readonly kind: "mcp" }
  | { readonly kind: "record"; readonly lens: Lens }
  | { readonly kind: "cost" }
  | { readonly kind: "registry" }
  | { readonly kind: "welcome"; readonly step?: GuideStep }
  | { readonly kind: "monitor" }
  | { readonly kind: "gallery" };

export const DEFAULT_VIEW: View = { kind: "talk", address: MAYOR };

export function toFragment(view: View): string {
  switch (view.kind) {
    case "talk":
      if (view.item !== undefined || view.session !== undefined) {
        const stretch = view.session === undefined ? "" : `${STRETCH}${String(view.session)}`;
        const item = view.item === undefined ? "" : `?${itemQuery(view.item)}`;
        return `#/talk/${escaped(view.address)}${stretch}${item}`;
      }
      return view.address === MAYOR ? "#/" : `#/talk/${escaped(view.address)}`;
    case "city":
      return "#/city";
    case "building":
      return `#/building/${escaped(view.address)}${view.section === "catalog" ? "/catalog" : ""}`;
    case "raise":
      return view.address === undefined ? "#/raise" : `#/raise/${escaped(view.address)}`;
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
      const version: Option.Option<B3Hash | null> = keys.endsWith("version")
        ? readB3(value("version"))
        : Option.some(null);
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
const readSeq = (raw: string): Option.Option<Seq> =>
  /^\d+$/.test(raw) ? Schema.decodeOption(Seq)(Number(raw)) : Option.none();
const readLens = (raw: string): Option.Option<RunLens> =>
  Option.fromNullishOr(RUN_LENSES.find((lens) => lens === raw));
const readStep = (raw: string): Option.Option<GuideStep> =>
  Option.fromNullishOr(GuideStep.literals.find((step) => step === raw));

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

const BARE: Readonly<Record<string, View>> = {
  "": DEFAULT_VIEW,
  talk: DEFAULT_VIEW,
  city: { kind: "city" },
  setup: { kind: "setup" },
  mcp: { kind: "mcp" },
  record: { kind: "record", lens: "ledger" },
  cost: { kind: "cost" },
  registry: { kind: "registry" },
  raise: { kind: "raise" },
  welcome: { kind: "welcome" },
  monitor: { kind: "monitor" },
  gallery: { kind: "gallery" },
};

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

export const PAGES: readonly string[] = Object.keys(BARE).filter((head) => head !== "");

export function page(name: string): Option.Option<View> {
  return Option.fromNullishOr(BARE[name] ?? OLD[name]);
}

function escaped(address: Address): string {
  return address.split("/").map(encodeURIComponent).join("/");
}

const decodeSegment = Option.liftThrowable(decodeURIComponent);

function unescaped(tail: string): Option.Option<string> {
  return Option.map(Option.all(tail.split("/").map(decodeSegment)), (segments) =>
    segments.join("/"),
  );
}

function readEscapedAddress(tail: string): Option.Option<Address> {
  return Option.flatMap(unescaped(tail), readAddress);
}

function talkIn(tail: string): Option.Option<View> {
  const cut = tail.lastIndexOf(STRETCH);
  const room = readEscapedAddress(cut < 0 ? tail : tail.slice(0, cut));
  if (cut < 0) return Option.map(room, (address) => ({ kind: "talk", address }));
  const digits = tail.slice(cut + 1);
  const session = readSeq(digits);
  return Option.flatMap(room, (address) =>
    Option.map(session, (began) => ({ kind: "talk", address, session: began })),
  );
}

function named(raw: string): string {
  return raw.replace(/^#*/, "").replace(/^\/*/, "");
}

export function fromFragment(raw: string): Option.Option<View> {
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
      return talkIn(tail);
    case "building":
    case "b": {
      const catalog = tail.endsWith("/catalog");
      const address = catalog ? tail.slice(0, -"/catalog".length) : tail;
      if (catalog) {
        return Option.map(readEscapedAddress(address), (building) => ({
          kind: "building",
          address: building,
          section: "catalog",
        }));
      }
      return Option.map(readEscapedAddress(address), (building) => ({
        kind: "building",
        address: building,
      }));
    }
    case "raise":
      return Option.map(readEscapedAddress(tail), (address) => ({ kind: "raise", address }));
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
      return Option.map(
        Option.all([readRunId(tail.slice(0, cut)), readLens(tail.slice(cut + 1))]),
        ([run, lens]) => ({ kind: "run", run, lens }),
      );
    }
    case "welcome":
      return Option.map(readStep(tail), (step) => ({ kind: "welcome", step }));
    default:
      return Option.none();
  }
}

function talkWith(path: string, query: string): Option.Option<View> {
  const head = ["talk/", "s/"].find((prefix) => path.startsWith(prefix));
  if (head === undefined) return Option.none();
  return Option.flatMap(talkIn(path.slice(head.length)), (view) =>
    view.kind === "talk"
      ? Option.map(readItem(query), (item): View => ({ ...view, item }))
      : Option.none(),
  );
}

export function unresolved(hash: string): Option.Option<string> {
  const name = named(hash);
  if (name === "" || Option.isSome(fromFragment(hash))) {
    return Option.none();
  }
  return Option.some(`#/${name}`);
}

export interface AddressBar {
  hash: string;
}

export function current(bar: Readonly<AddressBar>): Option.Option<View> {
  return fromFragment(bar.hash);
}

export function go(bar: AddressBar, view: View): void {
  bar.hash = toFragment(view);
}

export type WorkspaceView = Extract<View, { kind: "talk" }>;

export function readWorkspace(raw: string): WorkspaceView | null {
  const text = raw.trim();
  if (text === "") return null;
  const fragment = text.startsWith("#") ? text : URL.canParse(text) ? new URL(text).hash : null;
  if (fragment !== null) {
    const view = Option.getOrNull(fromFragment(fragment));
    if (view?.kind !== "talk") return null;
    return {
      kind: "talk",
      address: view.address,
      ...(view.session === undefined ? {} : { session: view.session }),
    };
  }
  return Option.getOrNull(
    Option.map(readAddress(text), (address): WorkspaceView => ({ kind: "talk", address })),
  );
}

export function workspaceDestination(
  view: WorkspaceView,
  buildings: readonly BuildingProgress[],
): View | null {
  const building = buildingOf(view.address);
  if (building === view.address) return buildingDestination(building, buildings).view;
  return buildings.some((held) => held.addr === building && !held.adoptable) ? view : null;
}

export function buildingOf(address: Address): Address {
  const slash = address.indexOf("/");
  return slash < 0 ? address : Address.make(address.slice(0, slash));
}

export type BuildingDestination =
  | {
      readonly kind: "building";
      readonly view: { readonly kind: "building"; readonly address: Address };
    }
  | {
      readonly kind: "adopt" | "build";
      readonly view: { readonly kind: "raise"; readonly address: Address };
    };

export function buildingDestination(
  addr: Address,
  buildings: readonly BuildingProgress[],
): BuildingDestination {
  const held = buildings.find((each) => each.addr === addr);
  if (held === undefined) return { kind: "build", view: { kind: "raise", address: addr } };
  if (held.adoptable) return { kind: "adopt", view: { kind: "raise", address: addr } };
  return { kind: "building", view: { kind: "building", address: addr } };
}

export function roomOf(address: Address): string {
  const slash = address.lastIndexOf("/");
  return slash < 0 ? address : address.slice(slash + 1);
}

export function roomIn(building: Address, name: string): Option.Option<Address> {
  const named = name.trim();
  return Schema.is(SessionName)(named)
    ? Option.some(Address.make(`${building}/${named}`))
    : Option.none();
}
