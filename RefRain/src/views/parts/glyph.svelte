<script lang="ts" module>
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  import type { Component } from "svelte";
  import type { ClassValue } from "svelte/elements";

  import Activity from "@lucide/svelte/icons/activity";
  import ArrowRightToLine from "@lucide/svelte/icons/arrow-right-to-line";
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import BookText from "@lucide/svelte/icons/book-text";
  import Box from "@lucide/svelte/icons/box";
  import Building2 from "@lucide/svelte/icons/building-2";
  import ChartLine from "@lucide/svelte/icons/chart-line";
  import Check from "@lucide/svelte/icons/check";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Circle from "@lucide/svelte/icons/circle";
  import Copy from "@lucide/svelte/icons/copy";
  import DoorOpen from "@lucide/svelte/icons/door-open";
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import FilePenLine from "@lucide/svelte/icons/file-pen-line";
  import FolderOutput from "@lucide/svelte/icons/folder-output";
  import GitBranch from "@lucide/svelte/icons/git-branch";
  import Hand from "@lucide/svelte/icons/hand";
  import Inbox from "@lucide/svelte/icons/inbox";
  import KeyRound from "@lucide/svelte/icons/key-round";
  import Layers from "@lucide/svelte/icons/layers";
  import Lock from "@lucide/svelte/icons/lock";
  import MessageSquare from "@lucide/svelte/icons/message-square";
  import Mic from "@lucide/svelte/icons/mic";
  import Pause from "@lucide/svelte/icons/pause";
  import Pin from "@lucide/svelte/icons/pin";
  import Search from "@lucide/svelte/icons/search";
  import Settings from "@lucide/svelte/icons/settings";
  import SlidersHorizontal from "@lucide/svelte/icons/sliders-horizontal";
  import Square from "@lucide/svelte/icons/square";
  import SquareTerminal from "@lucide/svelte/icons/square-terminal";
  import Wrench from "@lucide/svelte/icons/wrench";
  import X from "@lucide/svelte/icons/x";

  import type { GlyphName } from "./glyph";

  const DRAWN: Record<
    GlyphName,
    Component<{
      class?: ClassValue;
      strokeWidth?: number;
      absoluteStrokeWidth?: boolean;
      fill?: string;
    }>
  > = {
    talk: MessageSquare,
    city: Building2,
    record: BookText,
    cost: ChartLine,
    setup: SlidersHorizontal,
    hand: Hand,
    search: Search,
    chevron: ChevronRight,
    reveal: FolderOutput,
    ring: Circle,
    pulse: Activity,
    cross: X,
    check: Check,
    tool: Wrench,
    stop: Square,
    pause: Pause,
    capped: ArrowRightToLine,
    send: ArrowUp,
    layers: Layers,
    inbox: Inbox,
    settings: Settings,
    gate: Lock,
    sandbox: Box,
    terminal: SquareTerminal,
    propose: FilePenLine,
    pin: Pin,
    more: Ellipsis,
    branch: GitBranch,
    copy: Copy,
    door: DoorOpen,
    key: KeyRound,
    mic: Mic,
  };

  export type GlyphSize = "sm" | "md" | "key" | "stop";

  export interface GlyphProps {
    readonly name: GlyphName;
    readonly size?: GlyphSize;
    readonly solid?: boolean;
    readonly class?: ClassValue;
  }

  const BOX: Record<GlyphSize, string> = {
    sm: "size-glyph-sm",
    md: "size-glyph",
    key: "size-glyph-key",
    stop: "size-glyph-stop",
  };
</script>

<script lang="ts">
  const { name, size = "md", solid = false, class: klass }: GlyphProps = $props();
  const Drawn = $derived(DRAWN[name]);
</script>

<Drawn
  class={[BOX[size], klass]}
  strokeWidth={1.5}
  absoluteStrokeWidth
  {...solid ? { fill: "currentColor" } : {}}
/>
