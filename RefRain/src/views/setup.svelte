

<script lang="ts">
  
  
  import type { Snippet } from "svelte";

  import { QUERIES } from "../core/asking";
  import { setAutonomy } from "../core/commands";
  import { LANGS, endonym, say, type Key, type Lang } from "../core/lang";
  import { ui } from "../ui";
  import type { Autonomy, EndpointsAnswer, Proxying } from "../wire";
  import { ResidentId } from "../wire";
  import Machine from "./machine.svelte";
  import Glyph from "./parts/glyph.svelte";
  import Segmented from "./parts/segmented.svelte";
  import Release from "./release.svelte";
  import Automation from "./settings/automation.svelte";
  import Remote from "./settings/remote.svelte";
  import CityLayer from "./settings/city_layer.svelte";
  import ContextRung from "./settings/context_rung.svelte";
  import Rules from "./settings/rules.svelte";
  import Performance from "./settings/performance.svelte";
  import You from "./settings/you.svelte";
  import AdvancedSection from "./setup/advanced.svelte";
  import PrivacySection from "./setup/privacy/privacy.svelte";
  import { saveReceipt } from "./setup/appearance";
  import AppearanceSection from "./setup/appearance.svelte";
  import ColoursSection from "./setup/colours.svelte";
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
  import SearchCard from "./setup/search/search.svelte";
  import Agents from "./setup/agents.svelte";
  import Devices from "./settings/devices.svelte";
  import { HEADING, HINT, PREFERRED, WIDTH } from "./setup/groups";
  import type { SetupGroup } from "../core/route";

  type Setting = "proxying" | "autonomy" | "language";

  
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

  
  interface Props {
    readonly group?: SetupGroup | undefined;
    
    readonly titleId?: string | undefined;
    readonly endpoints?: EndpointsAnswer | undefined;
    readonly autonomy?: Autonomy | undefined;
    readonly onAutonomy?: ((setting: AutonomySetting) => void) | undefined;
    
    readonly dependency?: Snippet | undefined;
  }

  const { group = "you", titleId, endpoints, autonomy, onAutonomy, dependency }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const held = u.prefs.held;
  const keeper = u.prefs.keeper;
  const asking = u.conn.asking.ask(QUERIES.endpoints);
  const governance = u.conn.asking.ask(QUERIES.governance);

  const shown = $derived(group);

  const asked = $derived.by((): EndpointsAnswer | undefined => {
    const stated = $asking;
    return stated !== undefined && "endpoints" in stated ? stated.endpoints : undefined;
  });
  const askedAutonomy = $derived.by((): Autonomy | undefined => {
    const stated = $governance;
    return stated !== undefined && "governance" in stated ? stated.governance.autonomy : undefined;
  });
  
  const answer = $derived(endpoints ?? asked);
  const chosen = $derived(autonomy ?? askedAutonomy);
  const hint = $derived(HINT[shown]);
  const ruleNote = $derived(proxyingNote($held.proxying));

  const receipt = saveReceipt();
  const saved = receipt.saved;

  
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


<div class="flex min-w-0 flex-col gap-wide px-wide py-wide narrow:px-base">
  <header class="flex min-w-0 flex-col gap-tight border-b border-edge pb-base">
    <div class="flex flex-wrap items-center gap-base">
      <h2 id={titleId} class="text-title font-title" tabindex="-1">{say($lang, HEADING[shown])}</h2>
      {#if PREFERRED.includes(shown)}
        <Kept keeper={$keeper} />
      {/if}
    </div>
    {#if hint !== null}
      <p class="text-note text-text-quiet">{say($lang, hint)}</p>
    {/if}
  </header>
  <div class={["flex min-w-0 flex-col gap-base", WIDTH[shown]]}>
    {#if shown === "you"}
      <You />
    {:else if shown === "accounts"}
      
      <div class="flex flex-col gap-wide">
        <div class="min-w-0">
          <ProviderDoor stepped />
        </div>
        <section class="flex min-w-0 flex-col gap-snug border-t border-edge pt-wide">
          <header class="flex min-w-0 flex-col gap-tight">
            <h3 class="text-label font-label text-text">{say($lang, "setup_step_models")}</h3>
            <p class="text-note text-text-faint">{say($lang, "setup_step_models_hint")}</p>
          </header>
          {#if answer !== undefined}
            <ModelChoice {answer} />
          {/if}
        </section>
        
        <section class="flex min-w-0 flex-col gap-snug border-t border-edge pt-wide">
          <header class="flex min-w-0 flex-col gap-tight">
            <h3 class="text-label font-label text-text">{say($lang, "setup_step_defaults")}</h3>
            <p class="text-note text-text-faint">{say($lang, "setup_step_defaults_hint")}</p>
          </header>
          <div class="grid grid-fit items-start gap-base">
            <div class="min-w-0 rounded-card bg-raised px-base py-snug">
              <EffortSection />
            </div>
            <CityLayer />
          </div>
        </section>
        <section class="flex min-w-0 max-w-talk flex-col gap-snug border-t border-edge pt-wide">
          <header class="flex min-w-0 flex-col gap-tight">
            <h3 class="text-label font-label text-text">{say($lang, "setup_step_search")}</h3>
            <p class="text-note text-text-faint">{say($lang, "setup_step_search_hint")}</p>
          </header>
          <SearchCard />
        </section>
      </div>
    {:else if shown === "agents"}
      <Agents />
    {:else if shown === "devices"}
      <Devices />
    {:else if shown === "run"}
      <div class="grid grid-fit items-start gap-base">
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
        <ContextRung />
      </div>
    {:else if shown === "rules"}
      <Rules />
    {:else if shown === "performance"}
      <Performance />
    {:else if shown === "automation"}
      <Automation />
    {:else if shown === "network"}
      <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
        <span class="text-label font-label text-text">{say($lang, "setup_network_default")}</span>
        <p class="text-note text-text-faint">{say($lang, "setup_network_new_only")}</p>
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
        {@render proxyingControl()}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
        {@render foot(ruleNote !== undefined ? say($lang, ruleNote) : undefined, "proxying")}
      </div>
    {:else if shown === "remote"}
      <Remote />
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
    {:else if shown === "colours"}
      <ColoursSection />
    {:else if shown === "keys"}
      <KeysSection />
    {:else if shown === "advanced"}
      <AdvancedSection />
    {:else if shown === "privacy"}
      <PrivacySection />
    {:else if shown === "about"}
      <Release />
    {/if}
  </div>
  <Toml />
</div>
