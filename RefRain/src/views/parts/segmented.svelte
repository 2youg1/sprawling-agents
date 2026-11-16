<script lang="ts" generics="V extends string">
  
  
  import type { Attachment } from "svelte/attachments";

  import { lookOf } from "./segmented";
  import type { SegmentedLook, SegmentedProps } from "./segmented";
  import Look from "./segmented.look.svelte";

  const props: SegmentedProps<V> = $props();
  const uid = $props.id();

  
  const drawn: Record<string, HTMLElement | undefined> = {};
  const holds: Record<string, Attachment<HTMLElement> | undefined> = {};

  const hold = (value: V): Attachment<HTMLElement> => {
    const kept = holds[value];
    if (kept !== undefined) return kept;
    const made: Attachment<HTMLElement> = (node) => {
      drawn[value] = node;
      return () => {
        if (drawn[value] === node) drawn[value] = undefined;
      };
    };
    holds[value] = made;
    return made;
  };

  const look: SegmentedLook = $derived(
    lookOf(props, uid, {
      focus: (value) => drawn[value]?.focus(),
      hold,
    }),
  );
</script>

<Look {...look} />
