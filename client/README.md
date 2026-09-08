# client — the browser client

The page a person opens against one running `sprawling` binary. It is
TypeScript, built by [bun](https://bun.sh) and Vite, and it lives outside
the cargo workspace on purpose: the wire in `crates/channels` is the whole
API, and a client written against it in another language is a supported
thing to build. This one replaces the Dioxus wasm client over v0.0.4; both
coexist until card 6.11 removes `crates/web`. Decisions and interfaces
are recorded in [`client-SPEC.md`](client-SPEC.md).

## Two runtime dependencies, and why only two

| Package | What it is for |
|---|---|
| `solid-js` | The views. Settled HTML becomes JSX with almost no transcription (`class`, plain boolean attributes), and a component is a function that runs once, so the DOM it draws is the DOM it declared. |
| `effect` | The effectful core: the socket ladder, enrolment, asking, pacing, wire decoding through generated `Schema`, and `Match` for exhaustive frame handling. It gives typed errors and branded values, which is how this repository's Rust side thinks. |

No router library: the hash routes are hand-written in `src/core/route.ts`,
which reads every fragment the previous client wrote and writes one
spelling. No UI kit: colour, type, spacing and shape come from
`src/theme.css`, which re-expresses the token tables of
`crates/web/src/theme.rs`, and that file is the only place a colour, size
or family literal may appear.

## The boundary: Effect for the effectful core, Solid for views

Effect is used narrowly and never wraps a view. The only seam between the
two is `src/core/bridge.ts`: an Effect `Stream` becomes a Solid signal, and
an `Effect` becomes a resource holding an `Exit`. Failure crosses that
seam as data, never as an exception, which is why the lint bans below are
possible at all.

## What the lint bans, and why

- `any` and every type assertion except `as const` — an `as` is a claim the
  compiler stops checking; construct the value instead (`Brand.refined`).
- `throw` and `try` — Effect owns failure; a value that can fail is an
  `Effect` or an `Either`.
- a `switch` that does not cover its union — the same rule as Rust's
  exhaustive `match`, so adding a variant is a red build, not a silent gap.
- unused variables, and `eslint-plugin-solid`'s reactivity rules, which turn
  Solid's one silent failure (destructured props) into a red build.

Every word a person reads comes from `src/lang.json`, in both languages,
keyed by the `snake_case` name of the `Msg` variant in `crates/web/src/lang.rs`.

## Firefox first

Firefox is the browser this is built and accepted in. Open every screen in
Firefox before anything else, and treat a Firefox-only defect as a defect.

## Building and checking

```bash
just build-web       # bun install --frozen-lockfile && bun run build  ->  target/web-dist
just check-client    # lint, typecheck, tests
cd client && bun run dev   # Vite dev server
```

`bun.lock` is the authority for every version; `trustedDependencies` is
absent, so no package runs a lifecycle script on install. `typecheck` runs
TypeScript 7 (the native compiler, installed as `typescript-native`);
the linter's parser needs the JS compiler API and reads `typescript` 6.0.3.
