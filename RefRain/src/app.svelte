<script lang="ts">
  import { Option } from "effect";
  import { onMount, tick } from "svelte";

  import { QUERIES } from "./core/asking";
  import { cancel } from "./core/commands";
  import { runInFront } from "./core/in_front";
  import { folded, keymap } from "./core/keys";
  import { HOLD_MS, pressedOf } from "./core/press";
  import type { Action } from "./core/keys";
  import { say } from "./core/lang";
  import { markOf, paintMark } from "./core/mark";
  import { TIERS, type Tier } from "./core/prefs";
  import { DEFAULT_VIEW, MAYOR, current, toFragment } from "./core/route";
  import { markEndAtFrame, markStart } from "./core/timing";
  import type { View } from "./core/route";
  import { setUi, ui } from "./ui";
  import type { Opening } from "./ui";
  import Banners from "./views/banners.svelte";
  import Cheatsheet from "./views/parts/kbd.svelte";
  import Edge from "./views/edge.svelte";
  import Finder from "./views/finder.svelte";
  import { closeFinder, finderShown, openFinder, underOf } from "./views/finding.svelte";
  import { closeRight, rightItem } from "./views/inspect/open.svelte";
  import Notifier from "./views/notifier.svelte";
  import Pages from "./views/pages.svelte";
  import HomeSide from "./views/pages/home_side.svelte";
  import { homeSidesOf } from "./views/pages/home_side";
  import Palette from "./views/palette.svelte";
  import {
    closePalette,
    paletteArrival,
    paletteShown,
    togglePalette,
  } from "./views/palette_door.svelte";
  import Quit from "./views/quit.svelte";
  import Refusal from "./views/refusal.svelte";
  import Workspace from "./views/workspace.svelte";
  import SessionSwitcher from "./views/world/switcher.svelte";
  import SettingsPanel from "./views/settings/panel.svelte";
  import {
    closePanel,
    hostSettled,
    panelBeneath,
    panelGroup,
    pickGroup,
  } from "./views/settings/hosted.svelte";
  import { followViewport, watchColumns, type Columns } from "./views/shared/frame";
  import { motionOff } from "./views/shared/motion";
  import { launchOf, opensGuide } from "./views/welcome/guide";
  import { keepSheets, leaveSheet, openSheet, sheetsOpen } from "./views/sheets.svelte";

  const props: { opening: Opening } = $props();
  // svelte-ignore state_referenced_locally (the shell captures this value once at mount and never follows the prop)
  setUi(props.opening);
  const u = ui();
  const held = u.prefs.held;
  const lang = u.lang;
  const approvals = u.approvals;
  const belief = u.conn.belief;
  const linkState = u.conn.state;
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
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

  let view = $state.raw<View>(DEFAULT_VIEW);
  let sheetOpen = $state(false);
  let sessionSwitcherOpen = $state(false);

  let stopsWithoutRun = $state(0);

  let opener: HTMLElement | null = null;

  let arrived = false;

  const waiting = $derived($approvals?.length ?? 0);
  const working = $derived($belief.live.length > 0);

  const launch = $derived(
    $endpoints !== undefined && "endpoints" in $endpoints
      ? launchOf($endpoints.endpoints, $held.welcomed)
      : undefined,
  );

  function focusComposer(): void {
    const box = document.querySelector("main textarea");
    if (box instanceof HTMLTextAreaElement) box.focus();
  }

  function focusTitle(): void {
    const heading = document.querySelector("main h1");
    if (heading instanceof HTMLElement) {
      heading.tabIndex = -1;
      heading.focus();
    }
  }

  function settle(): void {
    const was = view;
    view = Option.getOrElse(current(u.bar), () => DEFAULT_VIEW);
    const back = hostSettled(was, view, arrived);
    const switchedSession =
      was.kind === "talk" &&
      view.kind === "talk" &&
      (was.address !== view.address || was.session !== view.session);
    if (switchedSession) {
      markStart("session_switch");
      markEndAtFrame("session_switch");
    }
    if (tierOnLanding !== null && view.kind === "talk") {
      u.prefs.setTier(tierOnLanding);
      tierOnLanding = null;
      markEndAtFrame("layer_drawn");
      void tick().then(focusComposer);
    } else if (arrived && view.kind !== "setup") {
      const intoComposer = switchedSession && view.kind === "talk" && view.session === undefined;
      void tick().then(() => {
        if (back !== null) {
          back.focus();
          return;
        }
        if (intoComposer) focusComposer();
        else focusTitle();
      });
    }
    arrived = true;
  }

  function follow(): void {
    const next = Option.getOrElse(current(u.bar), () => DEFAULT_VIEW);

    if (
      view.kind === "setup" ||
      next.kind === "setup" ||
      (view.kind === "talk" && next.kind === "talk") ||
      !("startViewTransition" in document) ||
      motionOff(document.documentElement)
    ) {
      settle();
      return;
    }
    void document.startViewTransition(() => {
      settle();
      return tick();
    });
  }

  function closeSheet(): void {
    sheetOpen = false;
    opener?.focus();
    opener = null;
  }

  function homeZen(): void {
    tierOnLanding = null;
    u.prefs.setTier("zen");
    u.go(DEFAULT_VIEW);
  }

  function homeTalk(): void {
    u.go(DEFAULT_VIEW);
  }

  let columns = $state<Columns>("twelve");
  let tierOnLanding: Tier | null = null;
  function cycleTier(): void {
    markStart("layer_drawn");
    if (columns === "one") {
      (sheetsOpen().includes("world") ? leaveSheet : openSheet)("world");
      markEndAtFrame("layer_drawn");
      return;
    }
    const next = TIERS[(TIERS.indexOf($held.tier) + 1) % TIERS.length] ?? "zen";
    if (view.kind === "talk") {
      u.prefs.setTier(next);
      markEndAtFrame("layer_drawn");
      focusComposer();
      return;
    }
    tierOnLanding = next;
    (view.kind === "setup" && panelBeneath().kind === "talk"
      ? closePanel
      : () => {
          u.go(DEFAULT_VIEW);
        })();
  }
  let peeking = $state(false);
  const tier = $derived(
    columns === "one"
      ? peeking || sheetsOpen().includes("world")
        ? "panorama"
        : "zen"
      : peeking
        ? "blend"
        : $held.tier,
  );

  let mailboxAsked = $state(0);

  let tierHeld: ReturnType<typeof setTimeout> | null = null;
  let exposing: ReturnType<typeof setTimeout> | null = null;

  function expose(on: boolean): void {
    if (exposing !== null) clearTimeout(exposing);
    exposing = null;
    if (!on) delete document.documentElement.dataset.expose;
    else exposing = setTimeout(() => void (document.documentElement.dataset.expose = ""), HOLD_MS);
  }

  function holdTier(): void {
    if (tierHeld !== null) return;
    tierHeld = setTimeout(() => {
      peeking = true;
    }, HOLD_MS);
  }

  function releaseTier(): void {
    if (tierHeld === null) return;
    clearTimeout(tierHeld);
    tierHeld = null;
    if (peeking) peeking = false;
    else cycleTier();
  }

  function letGo(event: KeyboardEvent): void {
    if (event.key === "Control" || event.key === "Meta") expose(false);
    const tierKey = folded(event.key) === folded(bindings.chord("tier.cycle")?.key ?? "");
    if (tierHeld !== null && (tierKey || !(event.ctrlKey || event.metaKey))) releaseTier();
  }

  function forget(): void {
    expose(false);
    if (tierHeld !== null) clearTimeout(tierHeld);
    tierHeld = null;
    peeking = false;
  }

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
        if (view.kind === "setup") closePanel();
        else u.go(GOES[action]);
        return;
      case "palette":
        togglePalette();
        return;
      case "session.switch":
        if (tier === "zen") sessionSwitcherOpen = !sessionSwitcherOpen;
        return;
      case "tier.cycle":
        holdTier();
        return;
      case "mailbox":
        mailboxAsked += 1;
        return;
      case "inspect":
        u.prefs.setPanel(rightItem() === null && !$held.panel);
        closeRight();
        return;
      case "help":
        opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        sheetOpen = true;
        return;
      case "composer.focus":
        focusComposer();
        return;
      case "finder":
        (finderShown() ? closeFinder : openFinder)();
        return;
      case "run.stop": {
        const going = runInFront($belief, view);
        if (going === undefined) stopsWithoutRun += 1;
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

    expose(
      (event.key === "Control" || event.key === "Meta") && !event.isComposing && !event.repeat,
    );
    if (event.key === "Escape") {
      if (sessionSwitcherOpen) sessionSwitcherOpen = false;
      else if (paletteShown()) closePalette();
      else if (sheetOpen) closeSheet();
      return;
    }
    if (event.repeat && tierHeld !== null) return;

    if (paletteShown() && !accel) return;
    const action = bindings.acting(pressedOf(event));
    if (action !== null) {
      event.preventDefault();
      act(action);
    }
  }

  $effect(() => u.conn.asking.ask(QUERIES.city).subscribe(() => undefined));

  let launched = false;
  $effect(() => {
    if (launch === undefined || launched) return;
    launched = true;
    if (opensGuide(launch) && view.kind === "talk") u.go({ kind: "welcome" });
  });

  $effect(() => {
    const name = $belief.city ?? say($lang, "nav_city");
    document.title = waiting > 0 ? `(${String(waiting)}) ${name}` : name;
    document.documentElement.lang = $lang;
  });

  $effect(() => {
    paintMark(document, markOf({ waiting, working, link: $linkState.kind }));
  });

  onMount(follow);
  onMount(keepSheets);
</script>

<svelte:window onhashchange={follow} onkeydown={keys} onkeyup={letGo} onblur={forget} />

<Notifier {view} />
<div
  {@attach (frame: HTMLElement) => watchColumns(frame, (next) => (columns = next))}
  {@attach (frame: HTMLElement) =>
    window.visualViewport === null ? undefined : followViewport(frame, window.visualViewport)}
  class="frame visual-viewport relative overflow-x-clip bg-page font-sans text-body text-text"
>
  <a
    href="#main"
    class="sr-only focus:not-sr-only focus:absolute focus:top-snug focus:left-snug focus:z-(--layer-skip-link) focus:rounded-control focus:bg-raised focus:px-base focus:py-snug focus:text-label focus:text-text"
  >
    {say($lang, "skip_main")}
  </a>
  <Banners />
  {#if view.kind === "setup"}
    {@const beneath = panelBeneath()}
    {#if beneath.kind === "talk"}
      <Workspace address={beneath.address} item={beneath.item} session={beneath.session} {tier} />
    {:else}
      <Pages view={beneath} />
    {/if}
    <SettingsPanel
      group={panelGroup(view)}
      {beneath}
      onPick={pickGroup}
      onClose={closePanel}
      onHome={homeZen}
    />
  {:else if view.kind === "talk"}
    <Workspace address={view.address} item={view.item} session={view.session} {tier} />
  {:else}
    <Pages {view} />
    {#if homeSidesOf(view)}
      <HomeSide onHome={homeTalk} />
    {/if}
  {/if}
  {#if view.kind !== "welcome"}
    <Edge {tier} onTier={cycleTier} onPeek={(on: boolean) => (peeking = on)} {mailboxAsked} />
  {/if}
  <Refusal {stopsWithoutRun} />
  <Quit />
  {#if paletteShown()}
    <Palette onClose={closePalette} arrival={paletteArrival()} />
  {/if}
  {#if sessionSwitcherOpen && tier === "zen"}
    <SessionSwitcher onClose={() => (sessionSwitcherOpen = false)} />
  {/if}
  {#if sheetOpen}
    <Cheatsheet seat="modal" onClose={closeSheet} />
  {/if}
  {#if finderShown()}
    <Finder under={underOf(view)} onClose={closeFinder} />
  {/if}
  <span class="sr-only">{toFragment(view)}</span>
</div>
