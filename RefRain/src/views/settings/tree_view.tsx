// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import {
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  onSettled,
  untrack,
} from "solid-js";

import { say } from "../../core/lang";
import { initialTyped, lineWalker } from "../../core/lines";
import { summary } from "../../core/monitor";
import { pressedOf } from "../../core/press";
import { LENSES, toFragment, type SetupGroup, type View } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { useBuildings } from "../shared/buildings";
import { foldsForGroup, foldsOf, toggled, type Folds } from "./folds";
import { buildingPages, byInitial, lookOf, stepped, type Page } from "./tree";
import TreeControls from "./tree_controls";

export interface SettingsTreeProps {
  readonly group: SetupGroup;
  readonly beneath: View;
  readonly onPick: (group: SetupGroup) => void;
}

interface Location {
  readonly group: SetupGroup;
  readonly beneath: View;
  readonly folds: Folds;
}

const RECORD: readonly Page[] = LENSES.map((lens) => ({
  kind: "page",
  view: { kind: "record", lens },
  word: `rec_${lens}`,
}));

export default function SettingsTree(props: SettingsTreeProps) {
  const { lang, conn } = ui();
  const { monitor } = conn;
  const language = storeValue(lang);
  const addresses = storeValue(useBuildings());
  const samples = storeValue(monitor.samples);
  const [location, setLocation] = createSignal<Location>();
  const uid = createUniqueId();
  const lines = lineWalker();
  let nav: HTMLElement | undefined;

  onSettled(() => monitor.watchSummary());

  const reading = createMemo(() => {
    const latest = samples().at(-1);
    if (latest === undefined) return null;
    const read = summary(latest);
    return `${read.cpu} · ${read.memory}`;
  });

  const standing = createMemo(() => {
    const saved = location();
    return (
      saved ?? {
        group: props.group,
        beneath: props.beneath,
        folds: foldsOf(props.group, props.beneath),
      }
    );
  });

  createEffect(
    () => ({ group: props.group, beneath: props.beneath }),
    ({ group, beneath }) => {
      const before = untrack(location);
      if (before?.group === group && toFragment(before.beneath) === toFragment(beneath)) return;
      const focused = nav?.ownerDocument.activeElement;
      const inTree = focused !== undefined && focused !== null && nav?.contains(focused);
      const folds =
        before === undefined || before.group === group
          ? foldsOf(group, beneath)
          : foldsForGroup(group, beneath);
      setLocation({ group, beneath, folds });
      if (inTree) onSettled(() => focusCurrent(focused));
    },
  );

  function focusCurrent(focused: Element): void {
    if (nav === undefined) return;
    const { activeElement, body } = nav.ownerDocument;
    if (activeElement === focused || activeElement === body) {
      nav.querySelector<HTMLElement>("[aria-current=true]")?.focus();
    }
  }

  function walk(event: KeyboardEvent): void {
    if (nav === undefined) return;
    const pressed = pressedOf(event);
    const entries = [...nav.querySelectorAll<HTMLElement>("[data-entry]")];
    const at = entries.findIndex((entry) => entry === nav?.ownerDocument.activeElement);
    const letter = initialTyped(pressed);
    const next =
      letter === null
        ? stepped(entries, at, lines(pressed, event.timeStamp))
        : byInitial(entries, at, letter, (entry) => entry.dataset.initial);
    if (next === undefined) return;
    event.preventDefault();
    next.focus();
  }

  const leaves = createMemo(() => ({ buildings: buildingPages(addresses()), record: RECORD }));
  const look = createMemo(() =>
    lookOf({ ...standing(), leaves: leaves(), reading: reading() }, uid, {
      words: (key) => say(language(), key),
      pick: props.onPick,
      toggle: (fold) => setLocation({ ...standing(), folds: toggled(standing().folds, fold) }),
      walk,
    }),
  );

  function hold(node: HTMLElement): void {
    nav = node;
  }

  return <TreeControls {...look()} hold={hold} />;
}
