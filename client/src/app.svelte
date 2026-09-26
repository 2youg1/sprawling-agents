<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The shell: one rail on the left, one content region, and the three
  // things that may float over them - a refusal, the palette, and the
  // sheet of keys. Which page shows is the address bar's decision, read
  // on every `hashchange`.
  //
  // The keys are not spelled here. `core/keys` holds the action, the
  // chord that reaches it and the person's own chord if they set one;
  // this file asks it which action a press was and does that one thing.
  // A stopped city is said once, here, as a banner over every page: the
  // city page used to draw a crescent nobody could read and dim itself
  // to 40%, which is a mood rather than a message.

  import { Option } from "effect";
  import { onMount, tick } from "svelte";

  import { QUERIES } from "./core/asking";
  import { halt, release } from "./core/commands";
  import { keymap } from "./core/keys";
  import type { Action } from "./core/keys";
  import { fill, say } from "./core/lang";
  import { markOf, paintMark } from "./core/mark";
  import { RAILS } from "./core/prefs";
  import { cityIsShut, CITY } from "./core/scope";
  import { DEFAULT_VIEW, MAYOR, current, toFragment } from "./core/route";
  import type { View } from "./core/route";
  import { setUi, ui } from "./ui";
  import type { Opening } from "./ui";
  import Building from "./views/building.svelte";
  import Banner from "./views/parts/banner.svelte";
  import Button from "./views/parts/button.svelte";
  import Cheatsheet from "./views/parts/kbd.svelte";
  import City from "./views/city.svelte";
  import Cost from "./views/cost.svelte";
  import Facts from "./views/facts.svelte";
  import LinkBanner from "./views/link_banner.svelte";
  import Mcp from "./views/mcp.svelte";
  import Notifier from "./views/notifier.svelte";
  import Palette from "./views/palette.svelte";
  import Rail from "./views/rail.svelte";
  import RecordView from "./views/record.svelte";
  import Refusal from "./views/refusal.svelte";
  import Registry from "./views/registry.svelte";
  import Run from "./views/run.svelte";
  import Setup from "./views/setup.svelte";
  import Talk from "./views/talk.svelte";
  import Welcome from "./views/welcome.svelte";
  import { motionOff } from "./views/shared/motion";

  // The Opening `main.ts` read off the page once. Taken as one prop
  // object and captured in one piece: the shell never follows its
  // fields reactively, which is what the suppression below is about.
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

  // The actions that move the address bar. Derived from `Action` rather
  // than written out, so an action named `go.*` in `core/keys` has to
  // land somewhere here before this file compiles.
  type GoAction = Extract<Action, `go.${string}`>;

  // Where each of them lands. Every other action moves the shell rather
  // than the address bar, and is answered by `act` below.
  //
  // **Keyed by `GoAction`, which is what makes this table checked**: a
  // key spelled wrong is not an action, and an action left out is a
  // missing property. A table keyed by `string` left the name of a
  // destination with two homes - this table and `ACTIONS` - and neither
  // could tell the other was wrong (roadmap B-77).
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
  let paletteOpen = $state(false);
  let sheetOpen = $state(false);
  // What opened the sheet, so closing it puts the focus back where the
  // person left it.
  let opener: HTMLElement | null = null;
  // Whether a page has been drawn yet: the first paint is not a
  // navigation, and the focus it lands on is the page's own choice.
  let arrived = false;

  const waiting = $derived($approvals.length);
  const working = $derived(Object.values($belief.runs).some((run) => run.doing.kind !== "frozen"));
  const halted = $derived(cityIsShut($belief.halted));
  const unsent = u.conn.unsent;
  // The attempt the ladder is on since the link was lost, held through
  // each `opening` between two waits so the banner does not blink off
  // for every try; null while the page is live or has never been.
  let lostAttempt = $state<number | null>(null);
  $effect(() => {
    const now = $linkState;
    if (now.kind === "backoff") lostAttempt = now.attempt + 1;
    else if (now.kind === "live" || now.kind === "refused") lostAttempt = null;
  });
  // How many runs this city cancelled. The wire carries no count of
  // what one halt froze, so this counts the runs whose own freeze says
  // `cancelled`, which is what a halt writes.
  const frozen = $derived(
    Object.values($belief.runs).filter(
      (run) => run.doing.kind === "frozen" && run.doing.completion === "cancelled",
    ).length,
  );
  // Whether this city can take a dispatch at all: a `main` model is
  // chosen. Until then the first page is the welcome, unless the person
  // has already walked it and asked to be left alone.
  const ready = $derived.by(() => {
    const answer = $endpoints;
    if (answer === undefined || !("endpoints" in answer)) return undefined;
    return answer.endpoints.chosen.some((chosen) => chosen.tag === "main");
  });

  // The box a person writes in, wherever the page put it. Reached by the
  // element it is rather than by a name this file would have to keep in
  // step with the composer.
  function focusComposer(): void {
    const box = document.querySelector("main textarea");
    if (box instanceof HTMLTextAreaElement) {
      box.focus();
    }
  }

  // After a navigation the new page's own heading takes the focus, so a
  // keyboard or a screen-reader user lands in the page that just opened
  // rather than back at the top of the rail (ux-upgrades A12). The
  // heading is focusable but stands outside the Tab order.
  function focusTitle(): void {
    const heading = document.querySelector("main h1");
    if (heading instanceof HTMLElement) {
      heading.tabIndex = -1;
      heading.focus();
    }
  }

  function settle(): void {
    view = Option.getOrElse(current(u.bar), () => DEFAULT_VIEW);
    if (arrived) void tick().then(focusTitle);
    arrived = true;
  }

  // One page replaces another. A swap is a cut; a view transition
  // carries the rail's current item and the page's title across, so
  // the eye keeps the thing it was already looking at.
  //
  // It wraps the read of the address bar rather than the write of it,
  // because `hashchange` arrives in a later task: a transition around
  // `go()` would finish before the page had changed. Doing it here
  // also covers the back button and every `<a href="#/…">` on the
  // page, which `go()` never sees.
  //
  // Firefox has not shipped the API, and a person who turned movement
  // off asked for no travel; both take the plain swap (client-SPEC
  // 9.0, rows 4 and 8).
  function follow(): void {
    if (!("startViewTransition" in document) || motionOff(document.documentElement)) {
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

  // The rail's posture is the person's, kept where their other
  // postures are kept (`core/prefs.ts`): a reload must not put the
  // column back on somebody who works with it away. Three postures,
  // cycled by one chord in the order `RAILS` states.
  function cycleRail(): void {
    const at = RAILS.indexOf($held.rail);
    u.prefs.setRail(RAILS[(at + 1) % RAILS.length] ?? "glyphs");
  }

  function typing(target: EventTarget | null): boolean {
    return (
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement ||
      (target instanceof HTMLElement && target.isContentEditable)
    );
  }

  function act(action: Action): void {
    switch (action) {
      // The eight arms that share a body are exactly `GoAction`, so
      // the lookup is checked here rather than guarded at run time.
      case "go.talk":
      case "go.city":
      case "go.mcp":
      case "go.record":
      case "go.cost":
      case "go.registry":
      case "go.setup":
      case "go.waiting":
        u.go(GOES[action]);
        return;
      case "palette":
        paletteOpen = !paletteOpen;
        return;
      case "rail.toggle":
        cycleRail();
        return;
      case "help":
        opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        sheetOpen = true;
        return;
      case "composer.focus":
        focusComposer();
        return;
      case "run.stop":
        u.send(halt(CITY));
        return;
      case "fork.here":
        // The entry under the hand is one page's own fact, so this
        // chord is answered where the entries are: the thread's own
        // window handler branches from it (`talk/thread.svelte`), and
        // the shell holds nothing to branch from.
        return;
    }
  }

  function keys(event: KeyboardEvent): void {
    const accel = event.ctrlKey || event.metaKey;
    if (event.key === "Escape") {
      if (paletteOpen) paletteOpen = false;
      else if (sheetOpen) closeSheet();
      else if ($held.rail === "named") u.prefs.setRail("glyphs");
      return;
    }
    // With the palette open, only a chord that holds the accelerator is
    // the shell's; everything else is being typed into its filter. The
    // text-field half of this rule lives in `core/keys`' `matches`.
    if (paletteOpen && !accel) {
      return;
    }
    const action = bindings.acting({
      key: event.key,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
      target: typing(event.target) ? "field" : "page",
    });
    if (action !== null) {
      event.preventDefault();
      act(action);
    }
  }

  // The run list is the ground every page stands on: asked here and
  // watched for the life of the shell, so it is refreshed whenever it
  // goes stale whatever page is open, and folded into what the page
  // believes.
  $effect(() => u.conn.asking.ask(QUERIES.city).subscribe(() => undefined));

  $effect(() => {
    if (ready === false && !$held.welcomed && view.kind === "talk") {
      u.go({ kind: "welcome" });
    }
  });

  // The document title and the tab's icon carry what a hidden tab most
  // needs to say: how many things wait for the person, and whether the
  // city is working at all.
  $effect(() => {
    // A city nobody has named is still a city, and the word for it is
    // the one the city page shows: the tab's title and the rail read
    // the same key as the page's own heading.
    const name = $belief.city ?? say($lang, "nav_city");
    document.title = waiting > 0 ? `(${String(waiting)}) ${name}` : name;
    document.documentElement.lang = $lang;
  });

  $effect(() => {
    paintMark(document, markOf({ waiting, working, link: $linkState.kind }));
  });

  onMount(follow);
</script>

<svelte:window onhashchange={follow} onkeydown={keys} />

<Notifier {view} />
<div class="relative flex h-screen bg-page font-sans text-body text-text">
  <a
    href="#main"
    class="sr-only focus:not-sr-only focus:absolute focus:top-snug focus:left-snug focus:z-30 focus:rounded-control focus:bg-raised focus:px-base focus:py-snug focus:text-label focus:text-text"
  >
    {say($lang, "skip_main")}
  </a>
  {#if view.kind !== "welcome"}
    <Rail {view} posture={$held.rail} onToggle={cycleRail} onPalette={() => (paletteOpen = true)} />
  {/if}
  <div class="flex min-h-0 min-w-0 flex-1 flex-col">
    {#if lostAttempt !== null}
      <LinkBanner attempt={lostAttempt} unsent={$unsent} onRetry={u.conn.retry} />
    {/if}
    {#if halted}
      <Banner
        text={say($lang, "halt_title")}
        {...frozen > 0 ? { detail: fill(say($lang, "halt_frozen"), { n: String(frozen) }) } : {}}
        weight="alert"
      >
        {#snippet action()}
          <Button label={say($lang, "city_release")} tone="secondary" onPress={() => u.send(release(CITY))} />
        {/snippet}
      </Banner>
    {/if}
    <main
      id="main"
      class="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto"
      aria-label={say($lang, "region_main")}
    >
      {#if view.kind === "talk"}
        <Talk address={view.address} />
      {:else if view.kind === "city"}
        <City />
      {:else if view.kind === "building"}
        <Building address={view.address} />
      {:else if view.kind === "run"}
        <Run run={view.run} />
      {:else if view.kind === "setup"}
        <Setup />
      {:else if view.kind === "mcp"}
        <Mcp />
      {:else if view.kind === "record"}
        <RecordView lens={view.lens} />
      {:else if view.kind === "cost"}
        <Cost />
      {:else if view.kind === "registry"}
        <Registry />
      {:else if view.kind === "welcome"}
        <Welcome />
      {:else if view.kind === "gallery"}
        <!-- The storybook and its fixture tables are a chunk of their
             own, fetched only when a person opens `#/gallery`, so the
             page every other route loads does not carry them. -->
        {#await import("./views/gallery.svelte")}
          <!-- Pending until the chunk lands; the render gate waits for
               this mark to go before it measures the page. -->
          <div data-pending></div>
        {:then gallery}
          <gallery.default />
        {/await}
      {/if}
    </main>
    <!-- Under every page, and outside `<main>` on purpose: it is a
         control surface rather than content, so it does not print,
         and a person tabbing through the page does not walk into
         seven unlabelled readings. The welcome walk is the one
         screen without it - nothing is running yet, and a strip of
         dashes would teach a first-time reader that this product
         shows them nothing. -->
    {#if view.kind !== "welcome"}
      <Facts />
    {/if}
  </div>
  <Refusal />
  {#if paletteOpen}
    <Palette onClose={() => (paletteOpen = false)} />
  {/if}
  {#if sheetOpen}
    <Cheatsheet onClose={closeSheet} />
  {/if}
  <span class="sr-only">{toFragment(view)}</span>
</div>
