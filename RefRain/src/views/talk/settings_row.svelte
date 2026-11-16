

<script lang="ts">
  import { untrack } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import type { PopoverBinding } from "../parts/popover";
  import { childOf, offersOf, orderOf, parentOf } from "./picker";
  import type { Combination, Section, Segment } from "./picker";
  import { pickerOf, startOf } from "./picker_look";
  import { inForce } from "./picker_scene";
  import type { PickerFacts, PickerHeld } from "./picker_scene";
  import { permissionsOf } from "./policy";
  import { RECENT_COMBINATIONS } from "./recent.svelte";
  import Sandbox from "./sandbox.svelte";
  import CatalogChip from "./catalog_chip.svelte";
  import { menuOf, sameBinding } from "./settings_row";
  import type { RowDraws, RowMenu, RowStarts, SettingsRowLook } from "./settings_row";
  import Look from "./settings_row.look.svelte";

  interface Props {
    readonly picker: PickerFacts;
    readonly room: Address | null;
    readonly draws: RowDraws;
    readonly kept: boolean;
    
    
    readonly menu?: RowStarts;
  }

  const { picker, room, draws, kept, menu: starts = "closed" }: Props = $props();
  const u = ui();
  const { lang } = u;
  const policy = u.policy;

  const startSegment = (from: RowStarts): Segment | null => {
    switch (from) {
      case "model":
      case "provider":
      case "level":
        return from;
      case "open":
      case "closed":
        return null;
    }
  };

  let menu = $state<RowMenu>(menuOf(untrack(() => starts)));
  let preview = untrack(() => starts) !== "closed";
  
  
  let drawnOpen = untrack(() => starts) !== "closed";
  let segment = $state<Segment | null>(startSegment(untrack(() => starts)));
  let pick = $state<PickerHeld["pick"]>(null);
  let parent = $state<string | null>(null);
  let query = $state("");
  let whole = $state<readonly Section[]>([]);
  let binding = $state<PopoverBinding | null>(null);
  let active = $state<string | null>(null);

  
  const pinnedNow = (): PickerHeld["pinned"] => {
    const offers = offersOf(picker.endpoints);
    const order = orderOf(offers);
    const offer = offers.find(
      (each) => each.endpoint === picker.chosen?.endpoint && each.model === picker.chosen.model,
    );
    return offer === undefined
      ? { first: undefined, second: undefined }
      : { first: parentOf(order, offer), second: childOf(order, offer) };
  };
  let pinned = $state<PickerHeld["pinned"]>(untrack(pinnedNow));
  let opened = $state<Combination | null>(untrack(() => inForce(picker)));
  
  
  let ended: Combination | null = null;

  
  const drawn: Record<"token" | "frame" | "permission" | "policyFrame", HTMLElement | undefined> = {
    token: undefined,
    frame: undefined,
    permission: undefined,
    policyFrame: undefined,
  };
  const keep =
    (slot: keyof typeof drawn): Attachment<HTMLElement> =>
    (node) => {
      drawn[slot] = node;
      return () => {
        if (drawn[slot] === node) drawn[slot] = undefined;
      };
    };
  const hold = {
    token: keep("token"),
    frame: keep("frame"),
    permission: keep("permission"),
    policyFrame: keep("policyFrame"),
  };
  
  
  let filter = $state<HTMLElement | undefined>(undefined);
  let first = $state<HTMLElement | undefined>(undefined);
  const holdFilter: Attachment<HTMLElement> = (node) => {
    filter = node;
    return () => {
      if (filter === node) filter = undefined;
    };
  };
  const holdFirst: Attachment<HTMLElement> = (node) => {
    first = node;
    return () => {
      if (first === node) first = undefined;
    };
  };

  
  $effect(() => {
    if (menu !== "policy" || first === undefined) return;
    if (preview) {
      preview = false;
      return;
    }
    first.focus();
  });
  $effect(() => {
    if (menu !== "model" || filter === undefined) return;
    if (preview) {
      preview = false;
      return;
    }
    filter.focus({ preventScroll: true });
  });

  const closePicker = (focus: "token" | "stay"): void => {
    menu = null;
    binding = null;
    query = "";
    if (ended !== null) RECENT_COMBINATIONS.keep(ended);
    ended = null;
    if (focus === "token") queueMicrotask(() => drawn.token?.focus());
  };

  const look: SettingsRowLook = $derived({
    catalog,
    facts: draws === "everything" ? facts : undefined,
    notLive: kept ? say($lang, "talk_not_live") : undefined,
    model:
      draws === "everything"
        ? pickerOf(
            $lang,
            picker,
            {
              open: menu === "model",
              segment,
              pick,
              parent,
              pinned,
              opened,
              query,
              whole,
              kept: RECENT_COMBINATIONS.kept,
              binding,
              active,
            },
            {
              open: (from) => {
                preview = false;
                drawnOpen = false;
                segment = from;
                pick = null;
                parent = null;
                query = "";
                whole = [];
                pinned = pinnedNow();
                opened = inForce(picker);
                ended = null;
                menu = "model";
              },
              close: closePicker,
              hold: (change) => {
                if (change.pick !== undefined) pick = change.pick;
                if (change.parent !== undefined) parent = change.parent;
                if (change.query !== undefined) query = change.query;
                if (change.whole !== undefined) whole = change.whole;
              },
              keep: (combination) => {
                ended = combination;
              },
              bound: (next) => {
                if (untrack(() => sameBinding(binding, next))) return;
                const initial = untrack(() => binding === null);
                binding = next;
                if (initial)
                  next.pointColumn(
                    startOf(
                      picker,
                      untrack(() => segment),
                    ),
                  );
              },
              cursor: (id) => {
                active = id;
              },
              focusFilter: () => {
                preview = false;
                filter?.focus({ preventScroll: true });
              },
              holdToken: hold.token,
              holdFilter,
              holdFrame: hold.frame,
            },
          )
        : undefined,
    permissions:
      draws === "everything"
        ? permissionsOf($lang, $policy, menu === "policy", {
            choose: u.choosePolicy,
            toggle: () => {
              menu = menu === "policy" ? null : "policy";
            },
            close: (focus) => {
              menu = null;
              if (focus === "opener") drawn.permission?.focus();
            },
            inside: (target) =>
              target instanceof Node && drawn.policyFrame?.contains(target) === true,
            hold: { frame: hold.policyFrame, trigger: hold.permission, first: holdFirst },
          })
        : undefined,
  });

  
  const outside = (event: PointerEvent): void => {
    if (menu !== "model" || drawnOpen) return;
    if (event.target instanceof Node && drawn.frame?.contains(event.target) === true) return;
    look.model?.menu?.onOutside();
  };
</script>

<svelte:window onpointerdown={outside} />

{#snippet catalog()}
  {#if room !== null}<CatalogChip {room} />{/if}
{/snippet}

{#snippet facts()}
  <Sandbox {room} />
{/snippet}

{#if draws === "everything" || kept}
  <Look {...look} />
{/if}
