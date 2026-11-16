

<script lang="ts">
  
  
  import { fill, say } from "../../core/lang";
  import { WORKBENCH, resized, widest } from "../../core/workbench";
  import type { Divider } from "../../core/workbench";
  import { ui } from "../../ui";
  import { dividerLookOf } from "./divider";
  import type { DividerLook } from "./divider";
  import Look from "./divider.look.svelte";

  interface Props {
    readonly divider: Divider;
    
    
    readonly label: string;
    readonly controls: string;
    
    
    readonly column: number;
  }

  const { divider, label, controls, column }: Props = $props();

  const u = ui();
  const { lang } = u;
  const bench = u.prefs.workbench;

  
  const start = $derived(divider === 0 ? 1 : 1 + $bench[0].span);
  const span = $derived(divider === 0 ? $bench[0].span : $bench[1].span);

  function set(next: number): void {
    if (next !== span) u.prefs.setWorkbench(resized($bench, divider, next));
  }

  function reset(): void {
    u.prefs.setWorkbench(WORKBENCH);
  }

  
  function lineAt(grid: HTMLElement, clientX: number): number {
    const frame = grid.closest<HTMLElement>(".frame") ?? grid;
    const style = getComputedStyle(frame);
    const gap = Number.parseFloat(style.columnGap) || 0;
    let edge =
      frame.getBoundingClientRect().left + (Number.parseFloat(style.paddingLeft) || 0) - gap / 2;
    const lines = style.gridTemplateColumns.split(" ").map((track) => {
      const at = edge;
      edge += Number.parseFloat(track) + gap;
      return at;
    });
    const distances = [...lines, edge].map((at) => Math.abs(clientX - at));
    return distances.indexOf(Math.min(...distances)) + 1;
  }

  let dragging = $state(false);
  let gutter = $state<HTMLElement | undefined>(undefined);

  const look: DividerLook = $derived(
    dividerLookOf(
      {
        label: fill(say($lang, "world_divider"), { pane: label }),
        controls,
        span,
        widest: widest($bench, divider),
        dragging,
      },
      {
        set,
        reset,
        drag: (now) => {
          dragging = now;
        },
        follow: (clientX) => {
          const grid = gutter?.parentElement;
          if (grid === null || grid === undefined) return;
          set(lineAt(grid, clientX) - start);
        },
      },
    ),
  );
</script>


<div
  bind:this={gutter}
  class="row-[1/3] -ml-gutter flex w-gutter justify-self-start"
  style:grid-column="{column} / span 1"
>
  <Look {...look} />
</div>
