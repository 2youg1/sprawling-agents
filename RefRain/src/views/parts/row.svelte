<script module>
  import { landing, taken } from "./row";
  import type { RowListProps, Target } from "./row";

  export { RowList };

  const REACHABLE = 'a[href], button:not([disabled]), [tabindex="0"]';

  function targetOf(target: EventTarget | null): Target {
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
      return "text";
    }
    if (target instanceof HTMLSelectElement) {
      return "select";
    }
    return target instanceof HTMLElement && target.isContentEditable ? "editable" : "row";
  }

  function walk(
    event: KeyboardEvent & { readonly currentTarget: EventTarget & HTMLUListElement },
  ): void {
    const step = taken(targetOf(event.target), event.key);
    if (step === null) {
      return;
    }
    const rows = [...event.currentTarget.children];
    const from = rows.findIndex(
      (row) => event.target instanceof Node && row.contains(event.target),
    );
    if (from < 0) {
      return;
    }
    const next = landing(
      rows.map((row) => {
        const reach = row.querySelector(REACHABLE);
        return reach instanceof HTMLElement ? reach : undefined;
      }),
      step,
      from,
    );
    if (next === undefined) {
      return;
    }
    event.preventDefault();
    next.focus();
  }
</script>

<script lang="ts">
  import Look from "./row.look.svelte";
  import { lookOf } from "./row";
  import type { RowProps } from "./row";

  const props: RowProps = $props();
  const look = $derived(lookOf(props));
</script>

{#snippet RowList(props: RowListProps)}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <ul aria-label={props.label} onkeydown={walk}>
    {@render props.rows()}
  </ul>
{/snippet}

<Look {...look} />
