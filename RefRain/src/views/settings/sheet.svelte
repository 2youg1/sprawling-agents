

<script lang="ts">
  
  
  import { say } from "../../core/lang";
  import type { SetupGroup, View } from "../../core/route";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import Setup from "../setup.svelte";
  import Tree from "./tree.svelte";

  interface Props {
    readonly group: SetupGroup;
    readonly beneath: View;
    readonly onPick: (group: SetupGroup) => void;
    
    readonly onHome: () => void;
    
    readonly titleId: string;
  }

  const { group, beneath, onPick, onHome, titleId }: Props = $props();
  const { lang } = ui();

  let body = $state<HTMLElement | undefined>(undefined);

  
  let drawn: SetupGroup | null = null;
  $effect(() => {
    if (group === drawn) return;
    drawn = group;
    body?.scrollTo({ top: 0 });
  });
</script>


<div class="@container/sheet h-full min-h-0">
  <div class="flex h-full min-h-0 @max-lg/sheet:flex-col @max-lg/sheet:overflow-y-auto">
    <div
      class="flex w-[calc(var(--silver-side)-var(--spacing-gutter)/2)] shrink-0 flex-col gap-base overflow-y-auto border-r border-edge px-base py-wide @max-lg/sheet:w-full @max-lg/sheet:overflow-visible @max-lg/sheet:border-r-0 @max-lg/sheet:border-b @max-lg/sheet:py-base"
    >
      <div class="flex items-center gap-snug px-snug">
        
        <button
          type="button"
          class="-ml-tight flex h-control-sm min-w-0 items-center gap-tight rounded-control pr-snug text-note text-text-quiet transition-[background-color,color,scale] ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100"
          aria-label={say($lang, "settings_home_zen")}
          onclick={onHome}
        >
          <Glyph name="chevron" size="sm" class="shrink-0 rotate-180" />
          <span class="truncate">{say($lang, "settings_home_zen")}</span>
        </button>
      </div>
      <p class="px-snug text-label font-label text-text">{say($lang, "nav_settings")}</p>
      <Tree {group} {beneath} {onPick} />
    </div>
    
    <section
      bind:this={body}
      class="slide-then @container/page flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto @max-lg/sheet:overflow-visible"
      aria-labelledby={titleId}
    >
      
      {#key group}
        <div class="shift">
          <Setup {group} {titleId} />
        </div>
      {/key}
    </section>
  </div>
</div>
