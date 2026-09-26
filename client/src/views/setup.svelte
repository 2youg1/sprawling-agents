<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // Settings: nine groups, the group navigation down the left, the MCP
  // door among them, and the city's own `config.toml` folded under the
  // body instead of standing beside it. The page starts at the rail: the
  // navigation is two hundred points wide and each group caps its own
  // width by what it holds (client-SPEC 4-33, 4-36).
  //
  // A group is a signal rather than a route - `core/route.ts` is the
  // address bar's one authority - and MCP is the one entry that leaves
  // this page: its door stands in this navigation where its group stood
  // (ux-upgrades S3c-1). Every setting this page draws itself is one
  // card: a title, one line saying what the setting governs, the
  // control, and a foot with the constraint or state on the left and the
  // save receipt on the right (client-SPEC 4-36, ux-upgrades A2).

  import { tick, type Snippet } from "svelte";

  import { QUERIES } from "../core/asking";
  import { setAutonomy } from "../core/commands";
  import { LANGS, endonym, say, type Key, type Lang } from "../core/lang";
  import { toFragment } from "../core/route";
  import { ui } from "../ui";
  import type { Autonomy, EndpointsAnswer, Proxying } from "../wire";
  import { ResidentId } from "../wire";
  import Machine from "./machine.svelte";
  import Glyph from "./parts/glyph.svelte";
  import Segmented from "./parts/segmented.svelte";
  import Release from "./release.svelte";
  import AdvancedSection from "./setup/advanced.svelte";
  import { saveReceipt } from "./setup/appearance";
  import AppearanceSection from "./setup/appearance.svelte";
  import Decided from "./setup/decided.svelte";
  import Kept from "./setup/kept.svelte";
  import KeysSection from "./setup/keys.svelte";
  import ModelChoice from "./setup/models.svelte";
  import { PROXYINGS, proxyingNote } from "./setup/providers/draft";
  import SkillsSection from "./setup/skills.svelte";
  import Toml from "./setup/toml.svelte";
  import EffortSection from "./shared/effort.svelte";
  import GovernedSection from "./setup/governed.svelte";
  import ProviderDoor from "./shared/provider.svelte";
  import { HEADING, HINT, NAV, NAV_HERE, NAV_THERE, NAV_WEAR, PREFERRED, WIDTH } from "./setup/groups";
  import type { Group } from "./setup/groups";

  type Setting = "proxying" | "autonomy" | "language";

  // Who answers an approval, as the two settings a person picks between:
  // `delegate` carries an address on the wire, and the clerk is the only
  // resident this screen delegates to.
  type AutonomySetting = "owner" | "delegate";

  const AUTONOMIES = [
    ["owner", "autonomy_owner"],
    ["delegate", "autonomy_clerk"],
  ] as const satisfies readonly (readonly [AutonomySetting, Key])[];

  function autonomySetting(held: Autonomy): AutonomySetting {
    return typeof held === "string" ? held : "delegate";
  }

  function autonomyOf(setting: AutonomySetting): Autonomy {
    switch (setting) {
      case "owner":
        return "owner";
      case "delegate":
        return { delegate: ResidentId.make("hall/clerk") };
    }
  }

  // What a caller may hand this page in place of a city - the answers a
  // city would give, the survey the dependency group draws, and which
  // group is on screen. These are the props the Solid `SetupPage`
  // carried; every one is optional, so the shell's bare `<Setup />`
  // (`app.svelte`) asks its own questions and keeps its own group.
  interface Props {
    // Whether this layout is the page or a region inside one: a
    // document may have exactly one heading of the page's own rank.
    readonly rank?: "page" | "section" | undefined;
    readonly group?: Group | undefined;
    readonly onPick?: ((each: Group) => void) | undefined;
    readonly endpoints?: EndpointsAnswer | undefined;
    readonly autonomy?: Autonomy | undefined;
    readonly onAutonomy?: ((setting: AutonomySetting) => void) | undefined;
    // The dependency group's body: the real one asks this machine.
    readonly dependency?: Snippet | undefined;
  }

  const { rank = "page", group, onPick, endpoints, autonomy, onAutonomy, dependency }: Props =
    $props();

  const u = ui();
  const lang = u.lang;
  const held = u.prefs.held;
  const keeper = u.prefs.keeper;
  const asking = u.conn.asking.ask(QUERIES.endpoints);
  const governance = u.conn.asking.ask(QUERIES.governance);

  // The group on screen: the caller's when it pins one, else this page's
  // own last pick.
  let picked = $state.raw<Group>("accounts");
  const shown = $derived(group ?? picked);
  // The page's heading, focusable outside the Tab order (ux-upgrades A12).
  let heading = $state<HTMLHeadingElement | null>(null);

  const asked = $derived.by((): EndpointsAnswer | undefined => {
    const stated = $asking;
    return stated !== undefined && "endpoints" in stated ? stated.endpoints : undefined;
  });
  const askedAutonomy = $derived.by((): Autonomy | undefined => {
    const stated = $governance;
    return stated !== undefined && "governance" in stated ? stated.governance.autonomy : undefined;
  });
  // An injected answer wins over the question this page would ask.
  const answer = $derived(endpoints ?? asked);
  const chosen = $derived(autonomy ?? askedAutonomy);
  const hint = $derived(HINT[shown]);
  const ruleNote = $derived(proxyingNote($held.proxying));

  const receipt = saveReceipt();
  const saved = receipt.saved;

  function pick(next: Group): void {
    onPick?.(next);
    if (group !== undefined) return;
    picked = next;
    void tick().then(() => {
      heading?.focus();
    });
  }

  // The one write behind the autonomy control: a caller that owns the
  // question answers it itself; this page otherwise sends the command.
  // "Saved" is said once the answer the page reads holds the choice, not
  // when the frame leaves: a frame the link could not carry, or one the
  // city refused, never shows the receipt.
  let awaited = $state.raw<AutonomySetting | null>(null);
  $effect(() => {
    if (awaited === null || chosen === undefined || autonomySetting(chosen) !== awaited) return;
    awaited = null;
    receipt.landed("autonomy");
  });

  function settle(next: AutonomySetting): void {
    awaited = next;
    if (onAutonomy !== undefined) {
      onAutonomy(next);
      return;
    }
    u.send(setAutonomy("city", autonomyOf(next)));
  }
</script>

{#snippet foot(constraint: string | undefined, name: Setting)}
  <!-- The `{@render}` lines below each carry one suppression of
      `no-confusing-void-expression`, the way `parts/segmented.svelte`
      settles it. -->
  <div class="mt-tight flex items-baseline justify-between gap-base">
    {#if constraint !== undefined}
      <span class="text-note text-text-faint">{constraint}</span>
    {/if}
    {#if $saved === name}
      <span class="fade ml-auto inline-flex items-center gap-tight text-note text-text-quiet">
        <Glyph name="check" size="sm" class="shrink-0" />
        {say($lang, "setup_saved")}
      </span>
    {/if}
  </div>
{/snippet}

{#snippet proxyingControl()}
  <Segmented
    label={say($lang, "setup_network_default")}
    options={PROXYINGS.map(([setting, word]) => ({ value: setting, label: say($lang, word) }))}
    held={$held.proxying}
    onPick={(next: Proxying) => {
      u.prefs.setProxying(next);
      receipt.landed("proxying");
    }}
  />
{/snippet}

{#snippet autonomyControl()}
  {#if chosen !== undefined}
    <Segmented
      label={say($lang, "setup_autonomy")}
      options={AUTONOMIES.map(([setting, word]) => ({ value: setting, label: say($lang, word) }))}
      held={autonomySetting(chosen)}
      onPick={(next: AutonomySetting) => {
        settle(next);
      }}
    />
  {/if}
{/snippet}

{#snippet languageControl()}
  <Segmented
    label={say($lang, "setup_language")}
    options={LANGS.map((each) => ({ value: each, label: endonym(each) }))}
    held={$held.lang}
    onPick={(next: Lang) => {
      u.prefs.setLang(next);
      receipt.landed("language");
    }}
  />
{/snippet}

<div
  class="flex min-h-0 w-full flex-1 flex-col gap-wide px-wide py-wide @lg/page:flex-row @lg/page:items-start @lg/page:gap-section"
>
  <!-- Opaque and one level up: the fields scrolling under the stuck
  strip sit in positioned wrappers of their own, and would otherwise be
  drawn through it. -->
  <nav
    class="sticky top-0 z-1 flex shrink-0 flex-row gap-tight overflow-x-auto bg-page @lg/page:w-[200px] @lg/page:flex-col @lg/page:overflow-visible"
    aria-label={say($lang, "setup_groups")}
  >
    {#each NAV as entry (entry.kind === "group" ? entry.group : entry.kind)}
      {#if entry.kind === "group"}
        <button
          type="button"
          class={[NAV_WEAR, shown === entry.group ? NAV_HERE : NAV_THERE]}
          aria-current={shown === entry.group ? "page" : undefined}
          onclick={() => {
            pick(entry.group);
          }}
        >
          {say($lang, HEADING[entry.group])}
        </button>
      {:else}
        <a href={toFragment({ kind: "mcp" })} class={[NAV_WEAR, NAV_THERE]}>
          {say($lang, "nav_mcp")}
        </a>
      {/if}
    {/each}
  </nav>

  <!-- The group's header is the page frame's header (`parts/page`),
  spelled here because this page's title follows the group; the body and
  the city's own `config.toml` stand side by side once there is room,
  the file as the side panel of every group. -->
  <div class="flex min-w-0 flex-1 flex-col gap-wide">
    <header class="flex min-w-0 flex-col gap-tight border-b border-edge pb-base">
      <div class="flex flex-wrap items-center gap-base">
        {#if rank === "page"}
          <h1 class="text-title font-title" tabindex="-1" bind:this={heading}>
            {say($lang, HEADING[shown])}
          </h1>
        {:else}
          <h2 class="text-title font-title">{say($lang, HEADING[shown])}</h2>
        {/if}
        {#if PREFERRED.includes(shown)}
          <Kept keeper={$keeper} />
        {/if}
      </div>
      {#if hint !== null}
        <p class="text-note text-text-quiet">{say($lang, hint)}</p>
      {/if}
    </header>
    <div class="flex min-w-0 flex-col gap-wide @wide/page:flex-row @wide/page:items-start @wide/page:gap-section">
      <div class={["flex min-w-0 flex-1 flex-col gap-base", WIDTH[shown]]}>
        {#if shown === "accounts"}
          <div class="flex flex-col gap-wide">
            <!-- The form takes the conversation's 760 rather than a
                measure, because what a person pastes into it is a base
                URL and a key and 520 cut both off (client-SPEC 4-33,
                4-36). -->
            <div class="min-w-0 max-w-talk">
              <ProviderDoor />
            </div>
            <div class="flex flex-col gap-base">
              <h2 class="text-label font-label text-text-quiet">{say($lang, "setup_models")}</h2>
              {#if answer !== undefined}
                <ModelChoice {answer} />
              {/if}
            </div>
          </div>
        {:else if shown === "run"}
          <div class="grid grid-fit items-start gap-base">
            <div class="min-w-0 rounded-card bg-raised px-base py-snug">
              <EffortSection />
            </div>
            <GovernedSection />
            <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
              <span class="text-label font-label text-text">{say($lang, "setup_autonomy")}</span>
              <p class="text-note text-text-faint">{say($lang, "setup_autonomy_note")}</p>
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
              {@render autonomyControl()}
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
              {@render foot(undefined, "autonomy")}
              <Decided />
            </div>
          </div>
        {:else if shown === "network"}
          <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
            <span class="text-label font-label text-text">{say($lang, "setup_network_default")}</span>
            <p class="text-note text-text-faint">{say($lang, "setup_network_new_only")}</p>
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
            {@render proxyingControl()}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
            {@render foot(ruleNote !== undefined ? say($lang, ruleNote) : undefined, "proxying")}
          </div>
        {:else if shown === "tools"}
          {#if dependency !== undefined}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
            {@render dependency()}
          {:else}
            <Machine />
          {/if}
        {:else if shown === "skills"}
          <SkillsSection />
        {:else if shown === "appearance"}
          <div class="grid grid-fit items-start gap-base">
            <AppearanceSection />
            <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
              <span class="text-label font-label text-text">{say($lang, "setup_language")}</span>
              <p class="text-note text-text-faint">{say($lang, "setup_language_note")}</p>
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
              {@render languageControl()}
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
              {@render foot(undefined, "language")}
            </div>
          </div>
        {:else if shown === "keys"}
          <KeysSection />
        {:else if shown === "advanced"}
          <AdvancedSection />
        {:else if shown === "about"}
          <Release />
        {/if}
      </div>
      <div class="min-w-0 @wide/page:sticky @wide/page:top-0 @wide/page:w-tree @wide/page:shrink-0">
        <Toml />
      </div>
    </div>
  </div>
</div>
