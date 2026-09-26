# How a screen gets built here

The view layer is exempt from SPEC-first and from red-before-green. What
makes an interface good is cheap iteration, and charging a SPEC and a
failing test for every visual change buys correctness the view layer was
not losing while taxing the only thing it is short of.

**Exempt from the ceremony is not exempt from judgement.** This is what
replaces it.

## The loop

```bash
cd client && bun run dev     # the page, reloading as you edit
just build-web               # the bundle the binary embeds
cargo xtask render           # what the page actually drew
just check-client            # lint, typecheck, the client's own tests
```

A screen is finished when `cargo xtask render` is green against
`#/gallery` and `just check-client` passes.

## Rules that are not negotiable

**Every word comes from `client/src/lang.json`.** A view calls `say`; it
does not write a sentence. `cargo xtask wording` reads the positions
where a reader is handed words — in a `.svelte` view, a text node and a
spoken attribute such as `placeholder`, `title`, `alt` or an `aria-*`
label; in a `.ts` module, the parts of a refusal — and refuses a literal
in any of them. A proper noun that is the same word in both languages
cannot go in the table, and carries `wording-ok: <reason>` on its line
or the line above.

**Every colour comes from `client/src/theme.css`.** That file is the only
place in the repository allowed to name one. `cargo xtask color` scans
every other file for `oklch(`, `rgb(`, `#rrggbb` and their relatives, and
holds the tokens themselves to a set of properties in both lightings: an
eleven-rung grey ramp that climbs, no pure black or white, one hue axis
and its single complement, exactly two chroma ratios, and text that
reaches the APCA tier it claims on the brightest surface text is allowed
to sit on.

**Every state worth looking at has a fixture on `#/gallery`.** That route
is where a screen is judged, by a person and by the gate, so a state with
no fixture is a state nobody looks at twice.

**Every fixture states the width its subject is drawn at.** The route
is as wide as the window, and a row list or a notice stretched across a
wide screen says nothing about how it looks in the column it lives in.
`client/src/views/gallery/case.svelte` therefore draws a component at the
conversation's width unless the case says otherwise, a page at the
window it is a page of, and a container under test at its own width. The
last of these also draws the frame, because there the width is the point,
and an unframed narrow table reads as a broken page rather than as a
container being tested.

## What `cargo xtask render` asserts

It opens the built bundle's `#/gallery` in a headless Chromium-family
engine and measures what was drawn. The page is opened once per pass:
once at each width `xtask/src/render/pass.rs` names, because the columns
are laid out by container queries and a property that holds at one width
is a property about that width, then once in the light and once in a
forced-colour mode, at the width the product is most often read at. Each
pass checks that the page drew the lighting it asked for.

What it asserts, each the generalisation of a defect that shipped:

1. Every control a person can operate has an accessible name.
2. Every landmark says which region it is.
3. A page has exactly one first heading.
4. The geometry `browser::survey` reads is within bounds: one left edge
   for the regions in the main column, one first mark down a navigation
   column, and nothing drawn outside the box that holds it.
5. Every popover shows an option, no text is crushed below a readable
   width, and no key hint is drawn underlined.

It asserts a property, never a picture. A screenshot comparison fails on
a font hint and passes on a page that is wrong in a way nobody
photographed.

**A missing bundle, a missing engine and a gallery that drew nothing are
each a violation that says which one it was.** A gate that goes quiet
when it cannot find its subject is green for the wrong reason. The first
two name what supplies the missing part: `just build-web`, or a browser
at `SPRAWLING_BROWSER`. The engine is looked for in three places, each
an authority that already exists: the path in `SPRAWLING_BROWSER`, then
what `sprawling doctor` installed among its components, then the usual
install paths of Edge, Chrome and Chromium.

## What a settled screen is walked against

`render` measures properties and cannot see the rest, so the last walk is
a person with the running window and this list. One pass per fixture and
per real screen, in both lightings:

1. **Contrast.** `text-quiet` and `text-faint` hold the WCAG ratio for
   body text against the pane behind them, with the root's `data-theme`
   in either state, and large text and graphics hold the ratio for
   large text. The colour gate reads the tokens; only an eye reads which
   tier a screen put where.
2. **Chinese typography.** No line opens with a closing bracket or a
   full stop; numbers beside Chinese characters sit on the mono figure
   face; running text is no wider than the `talk` container.
3. **Motion, both states.** Under `data-motion="off"`, and with the
   machine asking for reduced motion and no `data-motion="on"`, every
   animation reaches its final state without transition and the typing
   cursor stops blinking. A reduced-motion user must lose decoration,
   never information.
4. **Focus.** Walk every screen with Tab alone: the ring is visible on
   every stop, never clipped, and returns to where it left when a
   dialog closes.
5. **Hit areas.** Icon buttons measure at least `control-sm` on a
   desktop pointer, and their `::before` expansion reaches the touch
   target.
6. **Empty states.** Shape, the sentence naming what is missing, and an
   action out. The action may be absent; the sentence may not.
7. **Notices.** A refusal with a form goes inline beside its field, one
   without a page goes to a toast, history sits in the drawer. The role
   matches the seat.
8. **Tables.** Figure columns are right-aligned on `w-figure`, id
   columns hold `min-w-[24ch]`, and nothing wraps mid-token.
9. **No overflow.** No box draws outside the box that holds it,
   including popovers at the viewport edge.
10. **Every word sourced.** The wording gate reads the source; the eye
    confirms the painted result, where a fixture value is the only
    exception.

## What is still a person's job

Whether it looks right. The gate measures where boxes landed and what a
screen reader would be told; it has no opinion about whether a screen is
worth looking at. Open `#/gallery` in a real browser and decide.
