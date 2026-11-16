<script lang="ts">
  import { onDestroy, tick } from "svelte";

  import { initialTyped, lineWalker } from "../../core/lines";
  import { pressedOf } from "../../core/press";
  import { summary } from "../../core/monitor";
  import { LENSES } from "../../core/route";
  import type { SetupGroup, View } from "../../core/route";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { useBuildings } from "../shared/buildings";
  import { foldsForGroup, foldsOf, toggled } from "./folds";
  import type { Folds } from "./folds";
  import { buildingPages, byInitial, lookOf, stepped } from "./tree";
  import type { Nest, Page } from "./tree";
  import Look from "./tree.look.svelte";

  interface Props {
    readonly group: SetupGroup;

    readonly beneath: View;
    readonly onPick: (group: SetupGroup) => void;
  }

  const { group, beneath, onPick }: Props = $props();

  const u = ui();
  const { lang } = u;
  const buildings = useBuildings();
  const samples = u.conn.monitor.samples;
  onDestroy(u.conn.monitor.watchSummary());
  const reading = $derived.by(() => {
    const latest = $samples.at(-1);
    if (latest === undefined) return null;
    const read = summary(latest);
    return `${read.cpu} · ${read.memory}`;
  });

  const uid = $props.id();
  // svelte-ignore state_referenced_locally (the folds the panel opened on start open; the person folds them after)
  let folds = $state.raw<Folds>(foldsOf(group, beneath));

  const leaves = $derived<Record<Nest, readonly Page[]>>({
    buildings: buildingPages($buildings),
    record: LENSES.map((lens) => ({
      kind: "page",
      view: { kind: "record", lens },
      word: `rec_${lens}`,
    })),
  });

  let nav: HTMLElement | null = null;
  const hold = (node: HTMLElement): (() => void) => {
    nav = node;
    return () => {
      nav = null;
    };
  };

  const lines = lineWalker();

  let drawn: SetupGroup | null = null;
  let stood: View | null = null;
  $effect(() => {
    if (group === drawn && beneath === stood) return;
    const first = drawn === null;
    const groupMoved = !first && group !== drawn;
    drawn = group;
    stood = beneath;
    folds = groupMoved ? foldsForGroup(group, beneath) : foldsOf(group, beneath);
    if (!nav?.contains(document.activeElement)) return;
    void tick().then(() => {
      nav?.querySelector<HTMLElement>("[aria-current=true]")?.focus();
    });
  });

  function walk(event: KeyboardEvent): void {
    if (nav === null) return;
    const pressed = pressedOf(event);
    const entries = [...nav.querySelectorAll<HTMLElement>("[data-entry]")];
    const at = entries.findIndex((each) => each === document.activeElement);
    const letter = initialTyped(pressed);
    const next =
      letter === null
        ? stepped(entries, at, lines(pressed, event.timeStamp))
        : byInitial(entries, at, letter, (each) => each.dataset.initial);
    if (next === undefined) return;
    event.preventDefault();
    next.focus();
  }

  const look = $derived(
    lookOf({ group, beneath, folds, leaves, reading }, uid, {
      words: (key) => say($lang, key),
      pick: onPick,
      toggle: (fold) => {
        folds = toggled(folds, fold);
      },
      walk,
      hold,
    }),
  );
</script>

<Look {...look} />
