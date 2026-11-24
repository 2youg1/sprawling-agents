// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { dynamic } from "@solidjs/web";
import Activity from "@lucide/solid/icons/activity";
import ArrowRightToLine from "@lucide/solid/icons/arrow-right-to-line";
import ArrowLeft from "@lucide/solid/icons/arrow-left";
import ArrowLeftRight from "@lucide/solid/icons/arrow-left-right";
import ArrowUp from "@lucide/solid/icons/arrow-up";
import BookText from "@lucide/solid/icons/book-text";
import Box from "@lucide/solid/icons/box";
import Building2 from "@lucide/solid/icons/building-complex";
import ChartLine from "@lucide/solid/icons/chart-line";
import Check from "@lucide/solid/icons/check";
import ChevronRight from "@lucide/solid/icons/chevron-right";
import Circle from "@lucide/solid/icons/circle";
import Copy from "@lucide/solid/icons/copy";
import Crop from "@lucide/solid/icons/crop";
import DoorOpen from "@lucide/solid/icons/door-open";
import Ellipsis from "@lucide/solid/icons/ellipsis";
import Eye from "@lucide/solid/icons/eye";
import FilePenLine from "@lucide/solid/icons/file-pen-line";
import FolderOutput from "@lucide/solid/icons/folder-output";
import FolderTree from "@lucide/solid/icons/folder-tree";
import Grid2x2 from "@lucide/solid/icons/grid-2x2";
import GitBranch from "@lucide/solid/icons/git-branch";
import Hand from "@lucide/solid/icons/hand";
import Inbox from "@lucide/solid/icons/inbox";
import Info from "@lucide/solid/icons/info";
import KeyRound from "@lucide/solid/icons/key-round";
import Layers from "@lucide/solid/icons/layers";
import Lock from "@lucide/solid/icons/lock";
import LocateFixed from "@lucide/solid/icons/locate-fixed";
import MessageSquare from "@lucide/solid/icons/message-square";
import MessagesSquare from "@lucide/solid/icons/messages-square";
import Mic from "@lucide/solid/icons/mic";
import Pause from "@lucide/solid/icons/pause";
import Pin from "@lucide/solid/icons/pin";
import Plus from "@lucide/solid/icons/plus";
import RefreshCw from "@lucide/solid/icons/refresh-cw";
import Search from "@lucide/solid/icons/search";
import Settings from "@lucide/solid/icons/settings";
import SlidersHorizontal from "@lucide/solid/icons/sliders-horizontal";
import Square from "@lucide/solid/icons/square";
import SquareTerminal from "@lucide/solid/icons/square-terminal";
import Sun from "@lucide/solid/icons/sun";
import Wrench from "@lucide/solid/icons/wrench";
import X from "@lucide/solid/icons/x";
import ZoomIn from "@lucide/solid/icons/zoom-in";

import type { GlyphName, GlyphProps, GlyphSize } from "./glyph";

const DRAWN = {
  talk: MessageSquare,
  city: Building2,
  record: BookText,
  cost: ChartLine,
  setup: SlidersHorizontal,
  "app-grid": Grid2x2,
  "folder-tree": FolderTree,
  "session-list": MessagesSquare,
  refresh: RefreshCw,
  add: Plus,
  hand: Hand,
  search: Search,
  chevron: ChevronRight,
  "go-back": ArrowLeft,
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
  "switch-subject": ArrowLeftRight,
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
  "pdf-crop": Crop,
  "pdf-magnify": ZoomIn,
  "pdf-original": Eye,
  "pdf-brightness": Sun,
  "pdf-details": Info,
  "pdf-reference": LocateFixed,
} satisfies Record<GlyphName, typeof Activity>;

const BOX: Record<GlyphSize, string> = {
  sm: "size-glyph-sm",
  md: "size-glyph",
  key: "size-glyph-key",
  stop: "size-glyph-stop",
};

export default function Glyph(props: GlyphProps) {
  const Drawn = dynamic(() => DRAWN[props.name]);
  return (
    <Drawn
      class={[BOX[props.size ?? "md"], props.class].flat().filter(Boolean).join(" ")}
      strokeWidth={1.5}
      absoluteStrokeWidth
      fill={props.solid ? "currentColor" : "none"}
      aria-hidden="true"
    />
  );
}
