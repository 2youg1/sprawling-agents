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

## Three rules that are not negotiable

**Every word comes from `client/src/lang.json`.** A view calls `say`; it
does not write a sentence. `cargo xtask wording` reads the two positions
where JSX hands a reader words — a text node, and a spoken attribute like
`aria-label` — and refuses a literal in either. A proper noun that is the
same word in both languages cannot go in the table, and carries
`wording-ok: <reason>` on its line or the line above.

**Every colour comes from `client/src/theme.css`.** That file is the only
place in the repository allowed to name one. `cargo xtask color` scans
every other file for `oklch(`, `rgb(`, `#rrggbb` and their relatives, and
holds the tokens themselves to seven properties: eleven grey rungs that
climb, no pure black or white, one hue axis and its single complement,
exactly two chroma ratios, and text that reaches the APCA tier it claims
on the brightest surface text is allowed to sit on.

**Every state worth looking at has a fixture on `#/gallery`.** That route
is where a screen is judged, by a person and by the gate, so a state with
no fixture is a state nobody looks at twice.

## What `cargo xtask render` asserts

It opens the built bundle in a headless engine, waits for the client to
mount, and measures what was drawn. Five properties, each the
generalisation of a defect that shipped:

1. Every control a person can operate has an accessible name. Its first
   run found the composer's own text box announced as nothing.
2. Every landmark says which region it is.
3. A page has exactly one first heading.
4. A page has one left edge: every region in the main column starts at
   the same x.
5. Nothing is drawn outside the box that holds it.

It asserts a property, never a picture. A screenshot comparison fails on
a font hint and passes on a page that is wrong in a way nobody
photographed.

**A missing bundle, a missing engine and an empty route are each a skip
that says which one it was.** A gate that goes quiet when it cannot find
its subject is green for the wrong reason. The engine is looked for in
three places, and each is an authority that already exists:
`SPRAWLING_BROWSER`, then what `sprawling doctor` installed under
`~/.sprawling/components/`, then the desktop browsers.

## What is still a person's job

Whether it looks right. The gate measures where boxes landed and what a
screen reader would be told; it has no opinion about whether a screen is
worth looking at. Open `#/gallery` in a real browser and decide.
