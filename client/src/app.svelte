<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The shell: one twelve-column grid (client-SPEC 4-33), the page on its
  // columns, the three edge keys at the foot of the first column, and the
  // three things that may float over them - a refusal, the palette, and
  // the sheet of keys. Which page shows is the address bar's decision,
  // read on every `hashchange`; the conversation page lays itself out by
  // the tier (`views/workspace.svelte`), every other page takes the
  // columns right of the edge keys.
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
  import { cancel, release } from "./core/commands";
  import { runInFront } from "./core/in_front";
  import { keymap } from "./core/keys";
  import type { Action } from "./core/keys";
  import { fill, say } from "./core/lang";
  import { markOf, paintMark } from "./core/mark";
  import { TIERS } from "./core/prefs";
  import { cityIsShut, CITY } from "./core/scope";
  import { DEFAULT_VIEW, MAYOR, current, toFragment } from "./core/route";
  import type { View } from "./core/route";
  import { setUi, ui } from "./ui";
  import type { Opening } from "./ui";
  import Banner from "./views/parts/banner.svelte";
  import Button from "./views/parts/button.svelte";
  import Cheatsheet from "./views/parts/kbd.svelte";
  import Edge from "./views/edge.svelte";
  import LinkBanner from "./views/link_banner.svelte";
  import Notifier from "./views/notifier.svelte";
  import Pages from "./views/pages.svelte";
  import Palette from "./views/palette.svelte";
  import Refusal from "./views/refusal.svelte";
  import Workspace from "./views/workspace.svelte";
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

  // Where each of them lands; every other action moves the shell and is
  // answered by `act` below. Keyed by `GoAction`, so a key spelled wrong
  // is not an action and an action left out is a missing property.
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
  // How many times the stop key found no run in front of the person;
  // the corner answers each one.
  let stopsWithoutRun = $state(0);
  // What opened the sheet, so closing it puts the focus back where the
  // person left it.
  let opener: HTMLElement | null = null;
  // Whether a page has been drawn yet: the first paint is not a
  // navigation, and the focus it lands on is the page's own choice.
  let arrived = false;

  const waiting = $derived($approvals?.length ?? 0);
  const working = $derived($belief.live.length > 0);
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
  // How many runs this city cancelled.
  const frozen = $derived($belief.cancelled);
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

  // One page replaces another through a view transition that carries the
  // page's title across. It wraps the read of the address bar, not the
  // write, because `hashchange` arrives in a later task - which also
  // covers the back button and every `<a href="#/…">`. Firefox lacks the
  // API, and motion turned off asks for no travel: both take a plain swap.
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

  // The tier is the person's, kept where their other postures are kept
  // (`core/prefs.ts`), and cycled in the order `TIERS` states. Holding the
  // layers key - the edge key or its chord - shows the blend tier for as
  // long as it is held and changes nothing (client-SPEC 7E).
  function cycleTier(): void {
    const at = TIERS.indexOf($held.tier);
    u.prefs.setTier(TIERS[(at + 1) % TIERS.length] ?? "blend");
  }
  let peeking = $state(false);
  const tier = $derived(peeking ? "blend" : $held.tier);

  // How many times the mailbox chord asked for the drawer; the drawer
  // answers each one where it stands.
  let mailboxAsked = $state(0);

  // A press is held when its key stays down this long: the layers chord
  // becomes a look, and the accelerator alone draws every key's name.
  const HOLD_MS = 300;
  let tierHeld: ReturnType<typeof setTimeout> | null = null;
  let exposing: ReturnType<typeof setTimeout> | null = null;

  function expose(on: boolean): void {
    if (exposing !== null) clearTimeout(exposing);
    exposing = null;
    if (on) {
      exposing = setTimeout(() => {
        document.documentElement.dataset["expose"] = "";
      }, HOLD_MS);
    } else {
      delete document.documentElement.dataset["expose"];
    }
  }

  function holdTier(): void {
    if (tierHeld !== null) return;
    tierHeld = setTimeout(() => {
      peeking = true;
    }, HOLD_MS);
  }

  // The layers chord acts when it is let go: a tap changes the tier, and
  // a hold that already showed the blend tier only ends the look.
  function releaseTier(): void {
    if (tierHeld === null) return;
    clearTimeout(tierHeld);
    tierHeld = null;
    if (peeking) peeking = false;
    else cycleTier();
  }

  function letGo(event: KeyboardEvent): void {
    if (event.key === "Control" || event.key === "Meta") expose(false);
    if (tierHeld !== null && !(event.ctrlKey || event.metaKey)) releaseTier();
  }

  function forget(): void {
    expose(false);
    if (tierHeld !== null) clearTimeout(tierHeld);
    tierHeld = null;
    peeking = false;
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
      case "tier.cycle":
        holdTier();
        return;
      case "mailbox":
        mailboxAsked += 1;
        return;
      case "inspect":
        u.prefs.setPanel(!$held.panel);
        return;
      case "help":
        opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        sheetOpen = true;
        return;
      case "composer.focus":
        focusComposer();
        return;
      case "run.stop": {
        const going = runInFront($belief, view);
        if (going === undefined) stopsWithoutRun += 1;
        else u.send(cancel(going.run));
        return;
      }
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
    // The accelerator alone, held, draws every key's name; any other key
    // with it, or an input method's composition, is a chord and not a look.
    expose((event.key === "Control" || event.key === "Meta") && !event.isComposing && !event.repeat);
    if (event.key === "Escape") {
      if (paletteOpen) paletteOpen = false;
      else if (sheetOpen) closeSheet();
      return;
    }
    if (event.repeat && tierHeld !== null) return;
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

<svelte:window onhashchange={follow} onkeydown={keys} onkeyup={letGo} onblur={forget} />

<Notifier {view} />
<div class="frame relative h-screen overflow-hidden bg-page font-sans text-body text-text">
  <a
    href="#main"
    class="sr-only focus:not-sr-only focus:absolute focus:top-snug focus:left-snug focus:z-30 focus:rounded-control focus:bg-raised focus:px-base focus:py-snug focus:text-label focus:text-text"
  >
    {say($lang, "skip_main")}
  </a>
  {#if lostAttempt !== null || halted}
    <div class="col-[2/-1] row-start-1 flex flex-col gap-snug pb-base narrow:col-span-full">
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
    </div>
  {/if}
  {#if view.kind === "talk"}
    <Workspace address={view.address} {tier} />
  {:else}
    <Pages {view} />
  {/if}
  {#if view.kind !== "welcome"}
    <Edge {tier} onTier={cycleTier} onPeek={(on) => (peeking = on)} {mailboxAsked} />
  {/if}
  <Refusal {stopsWithoutRun} />
  {#if paletteOpen}
    <Palette onClose={() => (paletteOpen = false)} />
  {/if}
  {#if sheetOpen}
    <Cheatsheet onClose={closeSheet} />
  {/if}
  <span class="sr-only">{toFragment(view)}</span>
</div>
