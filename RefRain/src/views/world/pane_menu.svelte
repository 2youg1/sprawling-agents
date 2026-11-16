

<script lang="ts">
  
  
  import { tick } from "svelte";

  import { fill, say } from "../../core/lang";
  import { moved } from "../../core/workbench";
  import type { Pane, Side } from "../../core/workbench";
  import { ui } from "../../ui";
  import { drawnElements } from "./drawn";
  import { paneMenuLookOf, possible } from "./pane_menu";
  import type { Arranged, PaneMenuLook } from "./pane_menu";
  import Look from "./pane_menu.look.svelte";

  interface Props {
    readonly pane: Pane;
    
    readonly label: string;
    readonly arranged: Arranged;
  }

  const { pane, label, arranged }: Props = $props();

  const u = ui();
  const { lang } = u;
  const bench = u.prefs.workbench;
  const uid = $props.id();

  let open = $state(false);

  const drawn = drawnElements();

  const SIDES: readonly Side[] = ["left", "right"];
  function live(): HTMLElement[] {
    return SIDES.filter((side) => possible($bench, pane, side)).flatMap(
      (side) => drawn.get(side) ?? [],
    );
  }

  function show(): void {
    open = true;
    queueMicrotask(() => live()[0]?.focus());
  }

  
  function close(): void {
    open = false;
    void tick().then(() => drawn.get("trigger")?.focus());
  }

  const look: PaneMenuLook = $derived(
    paneMenuLookOf(
      { pane, bench: $bench, arranged, open, uid },
      {
        label,
        arrange: fill(say($lang, "world_arrange"), { pane: label }),
        left: say($lang, "world_move_left"),
        right: say($lang, "world_move_right"),
      },
      {
        live,
        show,
        close,
        leave: () => {
          open = false;
        },
        move: (side) => {
          u.prefs.setWorkbench(moved($bench, pane, side));
          close();
        },
        hold: drawn.hold,
      },
    ),
  );
</script>

<Look {...look} />
