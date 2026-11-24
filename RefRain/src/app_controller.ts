// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, flush, onSettled } from "solid-js";
import { storeValue } from "./core/solid_store";

import { QUERIES } from "./core/asking";
import { cancel } from "./core/commands";
import { runInFront } from "./core/in_front";
import { keymap } from "./core/keys";
import { pressedOf } from "./core/press";
import type { Action } from "./core/keys";
import { say } from "./core/lang";
import { markOf, paintMark } from "./core/mark";
import { MAYOR, toFragment } from "./core/route";
import { markEndAtFrame, markStart } from "./core/timing";
import type { View } from "./core/route";
import {
  launchView,
  shellVisit,
  type Navigation,
  type NavigationLayer,
  type Subject,
} from "./core/shell_visit";
import { ui } from "./ui";
import { requestedCitation, requestedProposal } from "./views/talk/attachments";
import {
  backEdgeActions,
  focusEdgeKey,
  NAVIGATION_KEY,
  type ChromeProps,
} from "./views/shell/chrome";
import { backContextTools } from "./views/parts/context_tools";
import { focusComposer, focusNavigation, focusRoute } from "./views/shell/focus";
import type { ShellLayout } from "./views/shell/layout";
import { heldKeys } from "./views/shell/held_keys";
import { closeFinder, finderShown, openFinder } from "./views/finding";
import { closeRight, rightItem } from "./views/inspect/open";
import { closePalette, paletteShown, togglePalette } from "./views/palette_door";
import {
  closePanel,
  hostSettled,
  panelBeneath,
  rememberSettingsOpener,
  setupPlacement,
} from "./views/settings/hosted";
import { motionOff } from "./views/shared/motion";
import { launchOf, opensGuide } from "./views/welcome/guide";
import { keepSheets } from "./views/sheets";

export function appController() {
  const u = ui();
  const held = storeValue(u.prefs.held);
  const lang = storeValue(u.lang);
  const approvals = storeValue(u.approvals);
  const belief = storeValue(u.conn.belief);
  const linkState = storeValue(u.conn.state);
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const bindings = keymap();

  type GoAction = Extract<Action, `go.${string}`>;

  const GOES: Readonly<Record<GoAction, View>> = {
    "go.talk": { kind: "talk", address: MAYOR },
    "go.waiting": { kind: "talk", address: MAYOR },
    "go.city": { kind: "city" },
    "go.setup": { kind: "setup" },
    "go.mcp": { kind: "mcp" },
    "go.record": { kind: "record", lens: "ledger" },
    "go.cost": { kind: "cost" },
    "go.registry": { kind: "registry" },
  };

  const visit = shellVisit();
  const placement = storeValue(setupPlacement);
  const [view, setView] = createSignal<View>(launchView(u.bar.hash, visit));
  const beneath = createMemo(() => {
    const current = view();
    return current.kind === "setup" ? panelBeneath(placement()) : current;
  });
  const [subject, setSubject] = createSignal<Subject>(visit.subject());
  const [navigation, setNavigation] = createSignal<Navigation>("closed");
  const [draft, setDraft] = createSignal(false);
  const [split, setSplit] = createSignal(visit.split());
  const [splitLine, setSplitLine] = createSignal(visit.splitLine());
  const [splitTop, setSplitTop] = createSignal(visit.splitTop());
  const [peeking, setPeeking] = createSignal(false);
  const [sheetOpen, setSheetOpen] = createSignal(false);
  const [sessionSwitcherOpen, setSessionSwitcherOpen] = createSignal(false);
  const [stopsWithoutRun, setStopsWithoutRun] = createSignal(0);
  const [mailboxAsked, setMailboxAsked] = createSignal(0);
  let opener: HTMLElement | null = null;
  let arrived = false;
  let routeChange = 0;
  const waiting = createMemo(() => approvals()?.length ?? 0);
  const working = createMemo(() => belief().live.length > 0);
  const launch = createMemo(() => {
    const registered = endpoints();
    return registered !== undefined && "endpoints" in registered
      ? launchOf(registered.endpoints, held().welcomed)
      : undefined;
  });

  const gestures = heldKeys({
    eligible: () => beneath().kind === "talk" && subject() === "session",
    densityKey: () => bindings.chord("tier.cycle")?.key,
    peek: setPeek,
    toggle: toggleDensity,
  });

  function settle(): void {
    const was = view();
    const next = launchView(u.bar.hash, visit);
    if (arrived && toFragment(was) === toFragment(next)) return;
    const back = hostSettled(was, next, arrived);
    const switched =
      was.kind === "talk" &&
      next.kind === "talk" &&
      (was.address !== next.address || was.session !== next.session);
    if (switched) {
      markStart("session_switch");
      markEndAtFrame("session_switch");
    }
    setView(next);
    if (next.kind === "talk") {
      visit.setTalk(next);
      if (next.item?.kind === "workspace-file") chooseSubject("files");
      else if (next.item !== undefined) chooseSubject("session");
    }
    if (switched) closeNavigation(false);
    if (arrived && next.kind !== "setup") {
      requestAnimationFrame(() => {
        if (view() !== next) return;
        if (back?.()) return;
        if (navigation() !== "closed") {
          focusNavigation();
          return;
        }
        if (
          was.kind === "record" &&
          next.kind === "record" &&
          document.activeElement?.matches('#main [role="tab"][aria-selected="true"]')
        )
          return;
        if (!(switched && focusComposer())) focusRoute();
      });
    }
    arrived = true;
  }
  function follow(): void {
    const ticket = ++routeChange;
    const next = launchView(u.bar.hash, visit);

    if (
      view().kind === "setup" ||
      next.kind === "setup" ||
      (view().kind === "talk" && next.kind === "talk") ||
      !("startViewTransition" in document) ||
      motionOff(document.documentElement)
    ) {
      settle();
      return;
    }
    const transition = document.startViewTransition(() => {
      if (ticket !== routeChange) return;
      settle();
      flush();
    });
    void transition.ready.catch(() => undefined);
  }

  function closeSheet(): void {
    setSheetOpen(false);
    opener?.focus();
    opener = null;
  }

  function homeTalk(): void {
    u.go(visit.talk());
  }

  function chooseSubject(next: Subject): void {
    if (subject() === next) return;
    setSubject(next);
    visit.setSubject(next);
    closeNavigation(false);
    gestures.cancelDensity();
  }

  function switchSubject(): void {
    chooseSubject(subject() === "files" ? "session" : "files");
    if (beneath().kind !== "talk") u.go(visit.talk());
  }

  function closeNavigation(returnFocus = true): void {
    if (navigation() === "closed") return;
    setNavigation("closed");
    if (returnFocus) requestAnimationFrame(() => focusEdgeKey(NAVIGATION_KEY));
  }

  function toggleNavigation(at?: number): void {
    if (navigation() !== "closed") {
      closeNavigation();
      return;
    }
    markStart("layer_drawn", at);
    setNavigation("list");
    if (beneath().kind !== "talk") u.go(visit.talk());
    else requestAnimationFrame(focusNavigation);
    markEndAtFrame("layer_drawn");
  }

  function selectNavigation(layer: NavigationLayer): void {
    setNavigation(layer);
  }

  function setPeek(on: boolean): void {
    setPeeking(on && held().tier === "zen");
  }

  function toggleDensity(): void {
    if (beneath().kind === "talk" && subject() === "session") {
      u.prefs.setTier(held().tier === "panorama" ? "zen" : "panorama");
    }
  }

  function showDraft(): void {
    if (split()) {
      focusComposer();
      return;
    }
    const next = !draft();
    setDraft(next);
    if (next) requestAnimationFrame(focusComposer);
  }

  function toggleSplit(): void {
    const next = !split();
    setSplit(next);
    visit.setSplit(next);
  }

  function resizeSplit(line: number): void {
    if (splitLine() === line) return;
    setSplitLine(line);
    visit.setSplitLine(line);
  }

  function resizeSplitTop(percentage: number): void {
    if (splitTop() === percentage) return;
    setSplitTop(percentage);
    visit.setSplitTop(percentage);
  }

  const chrome: ChromeProps = {
    get lang() {
      return lang();
    },
    get subject() {
      return subject();
    },
    get navigation() {
      return navigation() !== "closed";
    },
    get settingsOpen() {
      return view().kind === "setup";
    },
    onNavigation: toggleNavigation,
    onSubject: switchSubject,
    onSettings: (trigger) => {
      rememberSettingsOpener(trigger);
      if (view().kind === "setup") closePanel();
      else u.go({ kind: "setup" });
    },
  };
  const citation = storeValue(requestedCitation);
  const proposal = storeValue(requestedProposal);
  createEffect(
    () => [citation(), proposal()] as const,
    ([source, edit]) => {
      if (source === null && edit === null) return;
      chooseSubject("files");
      if (beneath().kind !== "talk") u.go(visit.talk());
    },
  );
  function act(action: Action): void {
    switch (action) {
      case "go.talk":
      case "go.city":
      case "go.mcp":
      case "go.record":
      case "go.cost":
      case "go.registry":
      case "go.waiting":
        u.go(GOES[action]);
        return;
      case "go.setup":
        if (view().kind === "setup") closePanel();
        else u.go(GOES[action]);
        return;
      case "palette":
        togglePalette();
        return;
      case "session.switch":
        if (held().tier === "zen") setSessionSwitcherOpen(!sessionSwitcherOpen());
        return;
      case "workspace.navigation":
        toggleNavigation();
        return;
      case "workspace.subject":
        switchSubject();
        return;
      case "tier.cycle":
        gestures.beginDensity();
        return;
      case "mailbox":
        setMailboxAsked((value) => value + 1);
        return;
      case "inspect":
        u.prefs.setPanel(rightItem() === null && !held().panel);
        closeRight();
        return;
      case "help":
        opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        setSheetOpen(true);
        return;
      case "composer.focus":
        focusComposer();
        return;
      case "finder":
        (finderShown() ? closeFinder : openFinder)();
        return;
      case "run.stop": {
        const going = runInFront(belief(), view());
        if (going === undefined) setStopsWithoutRun((value) => value + 1);
        else u.send(cancel(going.run));
        return;
      }
      case "fork.here":
      case "decide.yes":
      case "decide.edit":
      case "decide.no":
        return;
    }
  }

  function keys(event: KeyboardEvent): void {
    const accel = event.ctrlKey || event.metaKey;

    gestures.prepare(event);
    if (event.isComposing || event.defaultPrevented) return;
    if (event.key === "Escape") {
      if (backContextTools()) event.preventDefault();
      else if (sessionSwitcherOpen()) setSessionSwitcherOpen(false);
      else if (paletteShown()) closePalette();
      else if (sheetOpen()) closeSheet();
      else if (backEdgeActions()) event.preventDefault();
      else if (view().kind !== "setup" && navigation() === "list") closeNavigation();
      else if (view().kind !== "setup" && navigation() !== "closed") selectNavigation("list");
      else if (view().kind === "talk" && subject() === "files" && draft() && !split()) {
        setDraft(false);
        focusEdgeKey(NAVIGATION_KEY);
      }
      return;
    }
    if (event.repeat && gestures.holding()) return;

    if (paletteShown() && !accel) return;
    const action = bindings.acting(pressedOf(event));
    if (action !== null) {
      event.preventDefault();
      act(action);
    }
  }

  let launched = false;
  createEffect(
    () => launch(),
    (entry) => {
      if (entry === undefined || launched) return;
      launched = true;
      const now = view();
      if (
        opensGuide(entry) &&
        now.kind === "talk" &&
        now.item === undefined &&
        subject() === "session" &&
        navigation() === "closed" &&
        visit.workspace() === null
      )
        u.go({ kind: "welcome" });
    },
  );
  createEffect(
    () => [belief().city, lang(), waiting()] as const,
    ([city, language, count]) => {
      const name = city ?? say(language, "nav_city");
      document.title = count === 0 ? name : "(" + String(count) + ") " + name;
      document.documentElement.lang = language;
    },
  );
  createEffect(
    () => markOf({ waiting: waiting(), working: working(), link: linkState().kind }),
    (mark) => paintMark(document, mark),
  );
  onSettled(() => {
    settle();
    const sheets = keepSheets();
    const stop = u.conn.asking.ask(QUERIES.city).subscribe(() => undefined);
    window.addEventListener("hashchange", follow);
    window.addEventListener("keydown", keys);
    window.addEventListener("keyup", gestures.release);
    window.addEventListener("blur", gestures.forget);
    return () => {
      gestures.forget();
      sheets();
      stop();
      window.removeEventListener("hashchange", follow);
      window.removeEventListener("keydown", keys);
      window.removeEventListener("keyup", gestures.release);
      window.removeEventListener("blur", gestures.forget);
    };
  });
  const shell: ShellLayout = {
    get subject() {
      return subject();
    },
    get navigation() {
      return navigation();
    },
    get split() {
      return split();
    },
    get splitLine() {
      return splitLine();
    },
    get splitTop() {
      return splitTop();
    },
    get navigationCovered() {
      return view().kind === "setup";
    },
    get draft() {
      return draft();
    },
    get peeking() {
      return peeking();
    },
    resizeSplit,
    resizeSplitTop,
    selectNavigation,
    setDensity: toggleDensity,
    setPeek,
    toggleDraft: showDraft,
    toggleSplit,
    visit,
    closeNavigation,
    showDraft: () => {
      setDraft(true);
    },
  };
  return {
    view,
    beneath,
    subject,
    shell,
    placement,
    held,
    lang,
    chrome,
    homeTalk,
    mailboxAsked,
    stopsWithoutRun,
    closeSheet,
    sheetOpen,
    sessionSwitcherOpen,
    closeSwitcher: () => {
      setSessionSwitcherOpen(false);
    },
  };
}
