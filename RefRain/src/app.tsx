// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, onSettled, Show } from "solid-js";
import { appController } from "./app_controller";
import { say } from "./core/lang";
import { keymap, marks } from "./core/keys";
import { toFragment } from "./core/route";
import { storeValue } from "./core/solid_store";
import { finderState, underOf, closeFinder } from "./views/finding";
import { closePalette, paletteArrival, paletteShown, paletteState } from "./views/palette_door";
import { panelGroup, pickGroup, closePanel } from "./views/settings/hosted";
import { followViewport } from "./views/shared/frame";
import { homeSidesOf } from "./views/pages/home_side";
import { WorkspaceShell } from "./views/shell/workspace_shell";
import { Chrome } from "./views/shell/chrome";
import { focusTitle } from "./views/shell/focus";
import Mailbox from "./views/mailbox/mailbox";
import "./views/shell/shell.css";
import Banners from "./views/banners.solid";
import Cheatsheet from "./views/parts/cheatsheet";
import Finder from "./views/finder.solid";
import { watchNotifications } from "./views/notifier";
import Pages from "./views/pages";
import { HomeSide } from "./views/pages/home_side_view";
import Palette from "./views/palette.solid";
import Quit from "./views/quit_view";
import Refusal from "./views/refusal_view";
import SessionSwitcher from "./views/world/switcher_view";
import SettingsPanel from "./views/settings/panel_view";

export default function App() {
  const app = appController();
  watchNotifications(app.view);
  let viewportFrame: HTMLElement | undefined;
  onSettled(() => {
    if (viewportFrame !== undefined && window.visualViewport !== null)
      return followViewport(viewportFrame, window.visualViewport);
  });
  const palette = storeValue(paletteState);
  const finding = storeValue(finderState);
  const keys = keymap();
  const bound = storeValue(keys.bound);
  const mailboxHint = createMemo(() => {
    const chord = marks(bound().mailbox, keys.platform).join(" ");
    return (words: string) => (chord.length === 0 ? words : words + "  " + chord);
  });
  const workspace = createMemo(() => {
    const page = app.beneath();
    return page.kind === "talk" ? page : undefined;
  });
  const settings = createMemo(() => {
    const page = app.view();
    return page.kind === "setup" ? page : undefined;
  });
  return (
    <>
      <div
        ref={(frame) => {
          viewportFrame = frame;
        }}
        class="frame visual-viewport relative overflow-x-clip bg-page font-sans text-body text-text"
        data-refrain-session-shell={
          workspace() !== undefined && app.subject() === "session" ? "" : undefined
        }
      >
        <a
          href="#main"
          onClick={(event) => {
            event.preventDefault();
            focusTitle();
          }}
          class="sr-only focus:not-sr-only focus:absolute focus:top-snug focus:left-snug focus:z-(--layer-skip-link) focus:rounded-control focus:bg-raised focus:px-base focus:py-snug focus:text-label focus:text-text"
        >
          {say(app.lang(), "skip_main")}
        </a>
        <Banners />
        <Show
          when={workspace()}
          fallback={
            <>
              <Pages view={app.beneath()} />
              <Show when={homeSidesOf(app.beneath())}>
                <HomeSide onHome={app.homeTalk} />
              </Show>
            </>
          }
        >
          {(page) => (
            <WorkspaceShell
              address={page().address}
              item={page().item}
              session={page().session}
              tier={app.held().tier}
              shell={app.shell}
            />
          )}
        </Show>
        <Show when={settings()}>
          {(page) => (
            <SettingsPanel
              group={panelGroup(page(), app.placement())}
              beneath={app.beneath()}
              onPick={pickGroup}
              onClose={closePanel}
            />
          )}
        </Show>
        <Show when={app.view().kind !== "welcome"}>
          <nav
            class="edge-keys refrain-edge"
            data-refrain-edge=""
            aria-label={say(app.lang(), "region_edge")}
          >
            <Chrome
              {...app.chrome}
              mailbox={<Mailbox asked={app.mailboxAsked()} hint={mailboxHint()} />}
            />
          </nav>
        </Show>
        <Refusal stopsWithoutRun={app.stopsWithoutRun()} />
        <Quit />
        <Show when={paletteShown(palette())}>
          <Palette onClose={closePalette} arrival={paletteArrival(palette())} view={app.view()} />
        </Show>
        <Show when={app.sessionSwitcherOpen() && app.held().tier === "zen"}>
          <SessionSwitcher onClose={app.closeSwitcher} />
        </Show>
        <Show when={app.sheetOpen()}>
          <Cheatsheet seat="modal" onClose={app.closeSheet} />
        </Show>
        <Show when={finding()}>
          <Finder under={underOf(app.view())} onClose={closeFinder} />
        </Show>
        <span class="sr-only">{toFragment(app.view())}</span>
      </div>
    </>
  );
}
