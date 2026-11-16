<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  
  
  export type { Lens, TabsProps } from "./tabs";
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";

  import { lookOf, REACHABLE } from "./tabs";
  import type { TabsProps } from "./tabs";
  import Look from "./tabs.look.svelte";

  const props: TabsProps = $props();
  const uid = $props.id();

  
  const drawn: Record<string, HTMLElement | undefined> = {};
  const kept: Record<string, Attachment<HTMLElement> | undefined> = {};

  const keep = (id: string): Attachment<HTMLElement> => {
    const held = kept[id];
    if (held !== undefined) return held;
    const made: Attachment<HTMLElement> = (node) => {
      drawn[id] = node;
      return () => {
        if (drawn[id] === node) drawn[id] = undefined;
      };
    };
    kept[id] = made;
    return made;
  };

  
  let reachable = $state(true);

  const watch: Attachment<HTMLElement> = (node) => {
    let frame = 0;
    const measure = (): void => {
      frame = 0;
      reachable = node.querySelector(REACHABLE) !== null;
    };
    const observer = new MutationObserver(() => {
      if (frame === 0) frame = requestAnimationFrame(measure);
    });
    observer.observe(node, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["disabled", "href", "tabindex", "contenteditable", "type"],
    });
    measure();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  };

  const look = $derived(
    lookOf(props, uid, {
      focus: (id) => drawn[id]?.focus(),
      keep,
      watch,
      reachable,
    }),
  );
</script>

<Look {...look} />
