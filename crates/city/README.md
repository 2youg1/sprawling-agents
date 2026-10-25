# sprawling-city

The space and identity of a city. Buildings and rooms are the directory tree, and this package answers the questions that tree raises: how a building is raised and which building an address belongs to, who a resident is and what its identity adds to a run's frozen prefix, what a building's `RULES.toml` allows, how the three configuration layers resolve into one frozen configuration, which documents a building starts with, what its shelves hold, and what the city writes when it is first formed. The full list of modules and the question each answers is §1 of `crates/city/Spec.lean`.

Two directories travel inside the package because a crates.io archive carries the package directory and nothing else. `crates/city/templates/` holds the default text of the files a city and its buildings start with, such as `URBANITE.md`, `JOB.md` and `RULES.toml`. `crates/city/skills/` holds the skills shipped with the binary; `crates/city/build.rs` embeds every directory there that holds a `SKILL.md`, and the files directly under `crates/city/skills/` ride in the release archive instead.

Writing to disk and keeping history belong to `sprawling-storage`, and how a resident's identity is drawn belongs to the browser client. The one workspace package this one names is `kernel`.

## Who uses it

Two workspace packages depend on it: `sprawling`, the assembly root, and `sprawling-accounting`, the city's writer. The packages this one may name are its line in the `depmap` block of `ARCHITECTURE.md` section 3.

## The specification

`crates/city/Spec.lean` is the entry, and its parts live under `crates/city/spec/`: the configuration ladder, building ownership of an address, rule evaluation and write domains, the document lock, shelf precedence, skill audit and install, the governed documents and vocation are proved there. `ARCHITECTURE.md` section 11, *Specifications in Lean*, says how a SPEC is laid out.

## Building and checking

```bash
cargo nextest run -p sprawling-city   # the module tests beside the code
lake build crates.city.Spec           # this package's Lean specification
```

Some of this package's acceptance checks run end to end from `crates/sprawling/tests/`. `crates/city/Spec.lean` §16 says which test checks what and where it lives. `just test` and `just models` run every check for the whole workspace.
