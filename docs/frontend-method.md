# How a screen gets built here

This is the method for changing anything under `client/`: a new screen, a new state of an old one, or a control moved from one place to another. It is written for a person or an agent who has the repository open and a screen to change.

The view layer is exempt from the two rituals the Rust crates follow, writing the SPEC first and watching a test fail before the fix. What makes an interface good is cheap iteration, and a SPEC and a failing test for every visual change would tax that iteration to buy a kind of correctness the view layer was not losing. The exemption is not an exemption from judgement: four machine rules, one gate that measures the drawn page, and a walk a person makes through the running window take the place of the rituals.

## What to read first

- [`client/client-SPEC.md`](../client/client-SPEC.md) section 7, the **interaction contract**: for each part, the WAI-ARIA pattern it implements, what each key does, the exact `aria-*` values, and where the focus goes when the part closes. This half of the client is not exempt from SPEC-first, because a keyboard user cannot see that a screen looks right.
- `client/src/views/parts/`, the controls the client already has — button, field, table, segmented control, combobox, popover, dialog, tooltip and the rest. A screen composes these parts and does not write a control of its own, because a second implementation of a control is where two behaviours begin to differ.
- `client/src/theme.css`, the only file allowed to name a colour, and the tokens for spacing, radius, type size and control height that every class in a view is built from.
- `client/src/lang.json`, the only place a word a reader sees is written, in English and in Chinese.
- The fixture on `#/gallery` for the screen you are changing, under `client/src/views/gallery/`.

## The loop

```bash
cd client && bun run dev     # the page, reloading as you edit
just build-web               # the bundle the binary embeds, in target/web-dist
cargo xtask render           # where the boxes of #/gallery landed in a real engine
just check-client            # lint, typecheck, the client's own tests
```

`cargo xtask render --width <px>` narrows a run to one of the widths the gate knows, which is the fast loop while one layout is being settled. A screen is finished when `cargo xtask render` is green against `#/gallery`, `just check-client` passes, and a person has walked it as the last section describes.

## Building a screen, in order

1. **Settle it against the shipped stylesheet.** Start from the tokens in `theme.css` and the parts that exist. When the design needs a size, a gap or a colour the tokens do not have, the change is to `theme.css`, where the colour gate can judge it, and not to a value written into one view.
2. **Choose parts by what the person has to do.** A small exclusive choice that a person reads at a glance is a segmented control. A list longer than a settings card can hold as one row of equal cells — the font families this machine has installed, the editors a file can open in — is a native `<select>` styled with the control tokens, because it works the same way under a keyboard, a screen reader and a touch screen at any width. A long list the person searches is the combobox. A choice that cannot be made says why through `Tip` before the click, instead of refusing afterwards.
3. **Put every word in `lang.json`.** Add the key with both languages, then call `say`. A proper noun that is the same word in both languages still goes into the table, with the same spelling twice.
4. **Give every state a fixture on `#/gallery`.** Empty, loading, full, refused, too long, in both lightings. A state with no fixture is a state nobody looks at twice, and the gate cannot measure what the gallery does not draw.
5. **State the width each fixture is drawn at.** `client/src/views/gallery/case.svelte` draws a component at the conversation's width unless the case says otherwise, a page at the window it is a page of, and a container under test at exactly its own width, inside a dashed frame that says the box is a specimen rather than the page.
6. **Run the gate, then walk the screen.** `cargo xtask render` measures; the walk below is for everything a measurement cannot see.

## The four machine rules

**Every word comes from `client/src/lang.json`.** A view calls `say` and does not write a sentence. `cargo xtask wording` reads the places where a reader is handed words — in a `.svelte` view, a text node and a spoken attribute such as `placeholder`, `title`, `alt` or an `aria-*` label; in a `.ts` module, the parts of a refusal — and refuses a literal in any of them. A mark that is not a word, such as a separator glyph hidden from screen readers, carries `wording-ok: <reason>` on its line or the line above.

**Every colour comes from `client/src/theme.css`.** `cargo xtask color` scans every other file for `oklch(`, `rgb(`, `#rrggbb` and their relatives, and holds the tokens themselves to a set of properties in both lightings: an eleven-rung grey ramp that climbs, no pure black or white, one hue axis and its single complement, exactly two chroma ratios, and text that reaches the APCA tier it claims on the brightest surface text may sit on.

**The bundle stays inside its budget.** `cargo xtask budget` weighs the built client against `xtask/budgets.toml` and refuses a size badge in `docs/badges/` that no longer states the reading; `cargo xtask badge --write` rewrites the badge after a change that moved the size.

**The client's own lint rules hold.** eslint refuses `any`, `as`, `throw`, `try` and a `switch` that is not exhaustive. A value read from outside the page — a stored preference, a frame from the socket, the value of a `<select>` — is narrowed by looking it up in the list of values the code knows (`EDITORS.find((each) => each === word)`), never cast.

## What `cargo xtask render` asserts

It opens the built bundle's `#/gallery` in a headless Chromium-family engine and measures what was drawn. It opens the page once at each width `xtask/src/render/pass.rs` names — 768, 1280 and 2560 CSS pixels — because the columns are laid out by container queries and a property that holds at one width is a property about that width. Then it opens the page once in the light and once in a forced-colour mode, at the width the product is read at most, and each pass checks that the page drew the lighting it asked for.

Each assertion generalises a defect that shipped once:

1. Every control a person can operate has an accessible name.
2. Every landmark says which region it is.
3. A page has exactly one first heading.
4. The geometry `browser::survey` reads is within bounds: one left edge for the regions in the main column, one first mark down a navigation column, and nothing drawn outside the box that holds it.
5. Every popover shows an option, no text is crushed below a readable width, and no key hint is drawn underlined.

It asserts properties, never a picture. A screenshot comparison fails on a font hint and passes a page that is wrong in a way nobody photographed.

A missing bundle, a missing engine and a gallery that drew nothing are each a violation that names which one it was, because a gate that goes quiet when it cannot find its subject is green for the wrong reason. The first two name what supplies the missing part: `just build-web`, or a browser at `SPRAWLING_BROWSER`. The engine is looked for in the path in `SPRAWLING_BROWSER`, then among the components `sprawling doctor` installed, then at the usual install paths of Edge, Chrome and Chromium.

## The walk through the running window

`render` measures properties and cannot see the rest, so the last check is a person with the running window and this list, once per fixture and once per real screen, in both lightings.

1. **Contrast.** `text-quiet` and `text-faint` hold the WCAG ratio for body text against the pane behind them with the root's `data-theme` in either state, and large text and graphics hold the ratio for large text. The colour gate reads the tokens; only an eye reads which tier a screen put where.
2. **Chinese typography.** No line opens with a closing bracket or a full stop, numbers beside Chinese characters sit on the mono figure face, and running text is no wider than the `talk` container.
3. **Motion in both states.** Under `data-motion="off"`, and with the machine asking for reduced motion and no `data-motion="on"`, every animation reaches its final state without a transition and the typing cursor stops blinking. A reduced-motion user loses decoration and never loses information.
4. **Focus.** Walk every screen with Tab alone: the ring is visible on every stop, never clipped, and returns to where it left when a dialog closes.
5. **Hit areas.** Icon buttons measure at least `control-sm` on a desktop pointer, and their `::before` expansion reaches the 44-point touch target.
6. **Empty states.** Each has a shape, a sentence naming what is missing, and an action out. The action may be absent; the sentence may not.
7. **Notices.** A refusal with a form goes inline beside its field, one without a page goes to a toast, and history sits in the drawer. The role matches the seat.
8. **Tables.** Figure columns are right-aligned on `w-figure`, id columns hold `min-w-[24ch]`, and nothing wraps in the middle of a token.
9. **No overflow.** No box draws outside the box that holds it, popovers at the viewport edge included.
10. **Every word sourced.** The wording gate reads the source; the eye confirms the painted result, where a fixture value is the only exception.
11. **Whether it looks right.** The gate measures where boxes landed and what a screen reader is told; it has no opinion about whether a screen is worth looking at. Open `#/gallery` in a real browser and decide.

## What goes into the commit

The screen, its words in `lang.json`, its fixtures, and the badge when the size moved. When the change alters what a part owes a keyboard or a screen reader, the interaction contract in `client/client-SPEC.md` section 7 changes in the same commit, because that half is held to SPEC-first. When a screen offers a choice whose list rests on outside facts — which editors read a file-and-line link, which fonts are installed — the SPEC entry that cites where each fact came from changes with it.
