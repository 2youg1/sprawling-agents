

<script lang="ts">
  import { Kbd } from "../parts/kbd.svelte";
  import Tip from "../parts/tip.svelte";
  import type { CallLineLook } from "./call_line";
  import SteerPin from "./steer_pin.look.svelte";

  const look: CallLineLook = $props();
</script>

<Tip text={look.hint}>
  {#snippet children(id)}
    <button
      {...look.wire}
      aria-describedby={id}
      class={[
        "group relative grid h-control w-full grid-cols-[8ch_minmax(8ch,1fr)_minmax(9ch,max-content)_minmax(0,auto)] items-center gap-x-pane rounded-control px-snug text-left text-note narrow:grid-cols-[6ch_minmax(6ch,1fr)_auto_minmax(0,auto)] narrow:gap-x-snug",
        "before:absolute before:inset-y-snug before:left-0 before:w-hair before:rounded-pill",
        look.opened ? "bg-raised text-text before:bg-accent" : "text-text-quiet hover:wash",
      ]}
    >
      <span class="truncate text-text-faint">{look.kind}</span>
      <span class="flex min-w-0 items-center gap-base">
        <span
          class={[
            "min-w-[min(8ch,100%)] flex-1 truncate narrow:min-w-[min(6ch,100%)]",
            look.running ? "text-text" : "",
          ]}>{look.subject}</span
        >
        {#if look.failure !== null}
          <span class="min-w-0 max-w-[24ch] truncate text-alert narrow:max-w-[12ch]"
            >{look.failure}</span
          >
        {/if}
      </span>
      <span
        class="figure flex items-center justify-end gap-tight whitespace-nowrap text-text-faint"
      >
        {#if look.running}
          <span
            class="inline-block size-dot shrink-0 pulse rounded-pill bg-accent"
            aria-hidden="true"
          ></span>
        {/if}
        {#if look.time.kind === "landed"}
          {look.time.text}
        {:else if look.time.kind === "running"}
          <span class="text-text">{look.time.text}</span>
        {/if}
      </span>
      <span class="flex min-w-0 items-center gap-tight">
        {#if look.verdict !== null}
          <span
            class={["truncate", look.verdict.tone === "alert" ? "text-alert" : "text-text-faint"]}
            >{look.verdict.text}</span
          >
        {/if}
        <span
          data-line-keys
          class={[
            "items-center gap-hair narrow:hidden",
            look.opened ? "inline-flex" : "hidden group-focus-visible:inline-flex",
          ]}
          aria-hidden="true"
        >
          {#each look.moves as move (move)}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; typescript-eslint does not resolve exports of another .svelte module) -->
            {@render Kbd({ move })}
          {/each}
        </span>
        {#if look.pinned}
          <SteerPin />
        {/if}
      </span>
    </button>
  {/snippet}
</Tip>
