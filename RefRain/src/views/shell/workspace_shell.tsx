// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { dynamic } from "@solidjs/web";
import { createEffect, createMemo, createSignal, onSettled, Show } from "solid-js";

import { newestWorking } from "../../core/belief/live";
import { heldIn } from "../../core/belief/rooms";
import { POSTURE_WORD, type Doing } from "../../core/doing";
import { fill, say, type Key } from "../../core/lang";
import type { Tier } from "../../core/prefs";
import { MAYOR, buildingOf, roomOf, type ItemLink } from "../../core/route";
import type { Navigation, NavigationLayer } from "../../core/shell_visit";
import { stretchesOf } from "../../core/stretches";
import { scopedUi, ui } from "../../ui";
import type { Address, RoundsAnswer, Seq, SessionsAnswer } from "../../wire";
import { createInspector, rightItem, sharedInspector } from "../inspect/open";
import { followingIn } from "../inspect/reading";
import Right from "../right";
import { watchColumns, type Columns } from "../shared/frame";
import { leaveSheet, openSheet } from "../sheets";
import Talk from "../talk";
import Past from "../talk/past";
import { PAGE, type Seat } from "../talk/seat";
import { commitStateOf } from "../world/chosen";
import { Commits } from "../world/commits_view";
import PaneMenu from "../world/pane_menu.solid";
import { SessionList } from "./session_list";
import { gridColumn, layoutOf, oneColumnOf, type Lines } from "../workspace";
import { CONVERSATION_ID, NAVIGATION_ID, scopedId } from "./chrome";
import { workspaceDocuments } from "./documents";
import { Divider } from "./divider";
import { FileCanvas } from "./file_canvas";
import { FileLibrary } from "./file_library";
import type { ShellLayout } from "./layout";
import { LayoutTools, NavigatorFooter, toolActivity } from "./navigation_actions";
import { GaugeStrip, PlacePane, SessionPane } from "./panorama";
import { storeValue } from "../../core/solid_store";

export interface WorkspaceShellProps {
  readonly scope?: string;
  readonly address: Address;
  readonly session?: Seq | undefined;
  readonly tier: Tier;
  readonly item?: ItemLink | undefined;
  readonly shell: ShellLayout;
}

export function WorkspaceShell(props: WorkspaceShellProps) {
  const u = ui();
  const scope = scopedUi();
  const id = (part: string) => scopedId(part, props.scope);
  const Surface = dynamic(() => (props.scope === undefined ? "main" : "section"));
  const Heading = dynamic(() => (props.scope === undefined ? "h1" : "h2"));
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const held = storeValue(u.prefs.held);
  const bench = storeValue(u.prefs.workbench);
  const [columns, setColumns] = createSignal<Columns>("twelve");
  const [sessions, setSessions] = createSignal<SessionsAnswer>();
  const [answer, setAnswer] = createSignal<RoundsAnswer>();
  const [visitedPanorama, setVisitedPanorama] = createSignal(false);
  const inspection = props.scope === undefined ? sharedInspector : createInspector();
  const items = storeValue(inspection.items);
  const item = createMemo(() => rightItem(items()));
  const commit = storeValue(commitStateOf(scope));
  const docs = workspaceDocuments(props);
  let element: HTMLElement | undefined;
  let navHeading: HTMLHeadingElement | undefined;
  let navOpener: HTMLElement | undefined;
  let beforeNavigation: Navigation = "closed";
  let backFolder: (() => boolean) | undefined;

  const narrow = createMemo(() => columns() === "one");
  const files = createMemo(() => props.shell.subject === "files");
  const visibleConversation = createMemo(() => !files() || props.shell.draft || props.shell.split);
  const navigatorOpen = createMemo(
    () => props.shell.navigation !== "closed" && !props.shell.navigationCovered,
  );
  const navTitle = createMemo<Key>(() => {
    switch (props.shell.navigation) {
      case "history":
        return "world_commits";
      case "tools":
        return "shell_layout_tools";
      case "library":
        return "shell_library_tools";
      case "closed":
      case "list":
        return files() ? "world_files" : "world_sessions";
    }
  });

  function inNavigation(layer: NavigationLayer, trigger?: HTMLElement): void {
    if (trigger !== undefined) navOpener = trigger;
    props.shell.selectNavigation(layer);
  }
  const roomStretches = createMemo(() => {
    const stored = sessions();
    return stretchesOf(stored === undefined ? [] : [stored], (room) => heldIn(belief(), room));
  });
  const chosen = createMemo(() =>
    roomStretches().find((each) =>
      props.session === undefined ? each.current : each.line.began === props.session,
    ),
  );
  const past = createMemo(() =>
    props.session === undefined || chosen()?.current === true ? undefined : (chosen() ?? null),
  );
  const active = createMemo(() =>
    belief().live.reduce<Doing | undefined>(
      (latest, run) =>
        run.addr === props.address &&
        run.sessionBegan === (past() === undefined ? undefined : props.session)
          ? run.doing
          : latest,
      undefined,
    ),
  );
  const current = createMemo(() => {
    const target = past();
    return target === undefined
      ? (newestWorking(belief(), props.address) ?? heldIn(belief(), props.address).at(-1))
      : target?.runs.at(-1);
  });
  const gauge = createMemo(() => {
    const picked = commit();
    if (past() !== undefined) return current()?.run ?? null;
    return picked?.room === props.address ? picked.run : (current()?.run ?? null);
  });
  const following = createMemo(() => {
    const snapshot = answer();
    return snapshot === undefined ? [] : followingIn(snapshot);
  });
  const inspector = createMemo(
    () => item() !== null || (!narrow() && held().panel && following().length > 0),
  );
  const layout = createMemo(() =>
    narrow()
      ? oneColumnOf(props.tier, bench())
      : layoutOf(props.tier, inspector() ? "open" : "closed", bench()),
  );
  const column = createMemo<Seat>(() =>
    props.tier === "panorama" && !narrow() && !files()
      ? { kind: "workbench", gauged: gauge() }
      : PAGE,
  );
  const grid = (lines: Lines): string | undefined => (narrow() ? undefined : gridColumn(lines));
  const placeLabel = createMemo(() =>
    props.address === MAYOR
      ? (belief().city ?? say(lang(), "world_city"))
      : buildingOf(props.address),
  );
  const title = createMemo(() => {
    const base = props.address === MAYOR ? say(lang(), "talk_empty_mayor") : roomOf(props.address);
    const line = chosen()?.line;
    if (line?.parent === undefined || line.parent === null) return base;
    return line.branched_from === undefined || line.branched_from === null
      ? fill(say(lang(), "world_session_child"), { title: base })
      : fill(say(lang(), "world_session_branch"), {
          title: base,
          at: String(line.branched_from.at_seq),
        });
  });
  const staleSession = createMemo(() => {
    const at = props.session;
    return past() === undefined || at === undefined ? null : { at };
  });
  const area = createMemo(() => layout().right);
  const rightGrid = createMemo(() => {
    const right = area();
    return right === null ? undefined : grid(right);
  });
  const panorama = createMemo(() => !files() && !narrow() && layout().world === "beside");
  createEffect(panorama, (on) => {
    if (on) setVisitedPanorama(true);
  });

  createEffect(
    () => props.address,
    (address) => {
      setSessions(undefined);
      return u.conn.asking.ask({ sessions: { room: address } }).subscribe((response) => {
        setSessions(
          response !== undefined && "sessions" in response ? response.sessions : undefined,
        );
      });
    },
  );
  createEffect(
    () => current()?.run,
    (run) => {
      setAnswer(undefined);
      if (run === undefined) return;
      return u.conn.asking.ask({ rounds: { run } }).subscribe((response) => {
        setAnswer(response !== undefined && "rounds" in response ? response.rounds : undefined);
      });
    },
  );
  createEffect(
    () => props.item,
    (asked) => {
      if (asked?.kind === "call") inspection.openCall(asked);
      else if (asked?.kind === "document") inspection.openDocument(asked);
    },
  );
  createEffect(
    () => props.shell.navigation,
    (navigation) => {
      const was = beforeNavigation;
      beforeNavigation = navigation;
      if (navigation === "closed") {
        navOpener = undefined;
        return;
      }
      requestAnimationFrame(() => {
        if (!navigatorOpen() || props.shell.navigation !== navigation) return;
        if (navigation === "list" && was !== "list" && was !== "closed" && navOpener?.isConnected) {
          navOpener.focus({ preventScroll: true });
          navOpener = undefined;
        } else navHeading?.focus({ preventScroll: true });
      });
    },
  );
  createEffect(
    () => [files(), narrow(), inspector()] as const,
    ([fileMode, small, open]) => {
      if (props.scope !== undefined) return;
      if (!fileMode && small && open) openSheet("right");
      else if (!open) leaveSheet("right");
    },
  );
  onSettled(() => {
    const stopColumns =
      element === undefined
        ? undefined
        : watchColumns(element, (next) => {
            setColumns(next);
          });
    return () => {
      stopColumns?.();
    };
  });

  return (
    <Surface
      id={id("main")}
      ref={(node) => {
        element = node;
      }}
      class="workspace refrain-workspace col-span-full row-start-2 -m-margin grid min-h-0 grid-cols-subgrid grid-rows-[minmax(0,1fr)_auto] p-margin narrow:relative narrow:-mx-pane narrow:mt-0 narrow:-mb-pane narrow:px-pane narrow:pt-0 narrow:pb-pane"
      data-subject={props.shell.subject}
      data-draft={props.shell.draft ? "" : undefined}
      data-split={props.shell.split ? "" : undefined}
      style={{
        "--refrain-split-line": String(props.shell.splitLine),
        "--refrain-split-top": `${props.shell.splitTop}%`,
      }}
      aria-label={say(lang(), "region_main")}
    >
      <Heading tabindex="-1" class="sr-only">
        {title()}
      </Heading>
      <section
        id={id(NAVIGATION_ID)}
        class="refrain-navigation"
        data-side="left"
        data-layer={props.shell.navigation}
        aria-labelledby={id("workspace-navigation-heading")}
        aria-hidden={navigatorOpen() ? "false" : "true"}
        inert={!navigatorOpen()}
        onKeyDown={(event) => {
          if (
            event.key === "Escape" &&
            !event.isComposing &&
            !event.defaultPrevented &&
            files() &&
            props.shell.navigation === "list" &&
            backFolder?.()
          )
            event.preventDefault();
        }}
      >
        <header>
          <h2
            id={id("workspace-navigation-heading")}
            ref={(node) => {
              navHeading = node;
            }}
            data-navigation-heading=""
            tabindex="-1"
          >
            {say(lang(), navTitle())}
          </h2>
        </header>
        <div
          class="refrain-nav-body"
          hidden={
            !files() || (props.shell.navigation !== "list" && props.shell.navigation !== "library")
          }
        >
          <FileLibrary
            api={docs.api}
            lang={lang()}
            spaces={docs.spaces()}
            workspace={docs.workspace()}
            file={docs.opened()?.record.id ?? null}
            path={docs.opened()?.record.path}
            visible={files() && navigatorOpen() && props.shell.navigation === "list"}
            manage={props.shell.navigation === "library"}
            bindBack={(back) => {
              backFolder = back;
            }}
            onWorkspace={(id) => {
              docs.activate(id);
              inNavigation("list");
            }}
            onRegister={(space) => {
              docs.register(space);
              inNavigation("list");
            }}
            onOpen={docs.chooseFile}
          />
        </div>
        <div class="refrain-nav-body" hidden={props.shell.navigation !== "list" || files()}>
          <SessionList here={props.address} session={props.session} />
        </div>
        <Show when={props.shell.navigation === "history"}>
          <div class="refrain-nav-body">
            <Commits here={props.address} inspectOnly />
          </div>
        </Show>
        <Show when={props.shell.navigation === "tools"}>
          <LayoutTools
            lang={lang()}
            scope={props.scope}
            root={() => element}
            shell={props.shell}
            files={files()}
            panorama={props.tier === "panorama"}
            doing={active()}
          />
        </Show>
        <NavigatorFooter
          lang={lang()}
          files={files()}
          listed={props.shell.navigation === "list"}
          onNavigation={inNavigation}
          close={() => props.shell.closeNavigation()}
        />
      </section>
      <FileCanvas
        scope={props.scope}
        docs={docs}
        lang={lang()}
        visible={files()}
        session={props.session}
        began={chosen()?.line.began}
        targets={roomStretches()}
      />
      <Show when={props.shell.peeking && !files() && props.shell.navigation !== "tools"}>
        <aside class="refrain-activity" aria-label={say(lang(), "shell_active_run")}>
          <Show when={active()} fallback={say(lang(), "shell_idle_run")}>
            {(doing) => (
              <>
                <p>{say(lang(), POSTURE_WORD[doing().kind])}</p>
                <p>{toolActivity(doing())}</p>
              </>
            )}
          </Show>
        </aside>
      </Show>
      <Show when={visitedPanorama()}>
        <div class="refrain-world-pane refrain-world-session" hidden={!panorama()}>
          <PaneMenu pane="session" label={say(lang(), "world_session")} arranged="words" />
          <SessionPane
            lang={lang()}
            here={props.address}
            shown={past() === undefined ? undefined : gauge()}
          />
        </div>
        <div class="refrain-world-pane refrain-world-place" hidden={!panorama()}>
          <PaneMenu pane="commits" label={placeLabel()} arranged="words" />
          <PlacePane lang={lang()} here={props.address} />
        </div>
      </Show>
      <div
        id={id(CONVERSATION_ID)}
        data-refrain-session-content={!files() ? "" : undefined}
        class="refrain-conversation flex min-h-0 flex-col narrow:col-span-full row-[1/3]"
        style={{ "grid-column": files() ? undefined : grid(layout().talk) }}
        inert={!visibleConversation()}
      >
        <Show when={!files() && props.tier === "panorama"}>
          <div class="-mx-wide px-wide narrow:mx-0 narrow:px-0">
            <GaugeStrip lang={lang()} here={props.address} run={gauge()} />
          </div>
        </Show>
        <section
          class="relative -mx-wide flex min-h-0 flex-1 flex-col px-wide narrow:mx-0 narrow:px-0"
          aria-label={say(
            lang(),
            past() === undefined ? "region_conversation" : "region_past_session",
          )}
        >
          <Show
            when={staleSession()}
            fallback={<Talk address={props.address} band={false} seat={column()} />}
          >
            {(session) => (
              <Past
                address={props.address}
                session={session().at}
                stretch={past() ?? null}
                band={false}
                seat={column()}
              />
            )}
          </Show>
        </section>
      </div>
      <Show when={props.shell.split && files()}>
        <Divider
          scope={props.scope}
          stacked={narrow()}
          position={narrow() ? props.shell.splitTop : props.shell.splitLine}
          lang={lang()}
          onResize={narrow() ? props.shell.resizeSplitTop : props.shell.resizeSplit}
        />
      </Show>
      <Show
        when={files() ? item()?.kind === "changes" || item()?.kind === "document" : inspector()}
      >
        <div
          class={[
            "row-[1/3] -mt-margin -mr-margin -mb-margin flex min-h-0 border-l border-edge-panel narrow:absolute narrow:inset-0 narrow:z-(--layer-sheet) narrow:col-auto narrow:row-auto narrow:m-0 narrow:border-l-0",
            { sheet: !files() && narrow(), "refrain-file-inspector": files() },
          ]}
          data-side="right"
          style={{ "grid-column": files() ? undefined : rightGrid() }}
        >
          <Right
            following={following()}
            current={answer()}
            talk={props.address}
            inspection={inspection}
          />
        </div>
      </Show>
    </Surface>
  );
}
