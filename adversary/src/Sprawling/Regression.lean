-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Model

/-!
# What this adversary delivers: a Rust test.

A counterexample that stays in Lean is knowledge this repository does not have.
Rendering it as a test beside the code it accuses moves the knowledge into the
language that ships, and leaves nothing here that could grow into a second
authority.

The emitted test enters by the door `channels::server` uses —
`assembly::RunWorker::handle` — rather than by a socket. The adversary attacks
the binary over the wire because that is where an agent stands; the regression it
hands back runs in-process because a committed test should not need a port, and
the policy under accusation is the same on both paths.

The output has to survive `cargo fmt --check`, so this module agrees with
rustfmt rather than merely producing valid Rust.
-/

namespace Sprawling

/-- Each step with its position and the world it was asked in.

Walked once and reused, because every question the renderer asks — which
imports are needed, which statement a step becomes, which code is owed — is a
question about that pair. -/
private def sequenced (trace : Trace) : List (Nat × World × Any) :=
  go initialState 1 trace
where
  go (world : World) (index : Nat) : Trace → List (Nat × World × Any)
    | [] => []
    | packed :: rest =>
      (index, world, packed) :: go (advance world packed.act) (index + 1) rest

private def quoted (text : String) : String :=
  "\"" ++ String.join (text.toList.map escape) ++ "\""
where
  escape : Char → String
    | '"' => "\\\""
    | '\\' => "\\\\"
    | character => character.toString

/-- A distinct key per step, derived the way the product derives one.

Every state-changing command owns an `IdemKey`, so two steps sharing one would
make the second a replay of the first rather than a new action. -/
private def idemField (index : Nat) : String :=
  s!"idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b\"step{index}\"),"

private def scopeOf : Scope → String
  | .city => "channels::HaltScope::City"
  | .building addr => s!"channels::HaltScope::Building(Address::parse({quoted addr}).unwrap())"

private def verbOf : Action y → String
  | .raise .. => "CreateBuilding"
  | .work .. => "Dispatch"
  | .stop _ => "Halt"
  | .resume _ => "Release"
  | .seize => "Takeover"
  | .look => "Look is read from the Ledger and never handed to a worker"

private def fieldsOf (index : Nat) : Action y → List String
  | .raise addr template =>
    [ s!"addr: Address::parse({quoted addr}).unwrap(),"
    , s!"template: channels::TemplateName::parse({quoted template.name}).unwrap(),"
    , idemField index ]
  | .work addr session =>
    [ s!"addr: Address::parse({quoted addr}).unwrap(),"
    , "task: \"say something\".to_owned(),"
    , "goal: \"an answer\".to_owned(),"
    , "mode: channels::ModeTag::parse(\"build\").unwrap(),"
    , idemField index
    , s!"session: Some(kernel::SessionName::parse({quoted session}).unwrap()),"
    , "effort: None," ]
  | .stop scope => [s!"scope: {scopeOf scope},", idemField index]
  | .resume scope => [s!"scope: {scopeOf scope},", idemField index]
  | .seize => ["run: RunId::from_bytes([1u8; 16]),", idemField index]
  | .look => []

/-- One `handle` call, laid out the way rustfmt lays a method chain out. -/
private def handed (column : Nat) (index : Nat) (act : Action y) : List String :=
  let margin := "".pushn ' ' column
  [margin ++ ".handle(channels::Command::" ++ verbOf act ++ " {"]
    ++ (fieldsOf index act).map (fun field => margin ++ "    " ++ field)
    ++ [margin ++ "})"]

/-- The Rust name of a code the door prints.

Written out rather than derived from the string, so a code this adversary has
never asserted on cannot be rendered into a name that does not exist. -/
private def variant : Code → String
  | ⟨"E_INVALID_ARGS"⟩ => "InvalidArgs"
  | ⟨"E_GATE_DENIED"⟩ => "GateDenied"
  | ⟨"E_CONFIG_INVALID"⟩ => "ConfigInvalid"
  | ⟨"E_WIRE_MISMATCH"⟩ => "WireMismatch"
  | ⟨other⟩ => s!"UnknownToTheRenderer_{other}"

private def succeeded (world : World) (index : Nat) (act : Action y) : List String :=
  match act with
  | .look =>
    let owned := world.standingAddresses.map (fun addr => quoted addr ++ ".to_owned()")
    [ "    assert_eq!("
    , "        standing(&raised.ledger_dir),"
    , "        vec![" ++ String.intercalate ", " owned ++ "]"
    , "    );" ]
  | _ => ["    worker"] ++ handed 8 index act ++ ["        .unwrap();"]

/-- A refusal, in two statements rather than one.

Putting the call inside `assert_eq!` would leave its shape to whatever rustfmt
does inside a macro; taking the error out first keeps the formatting predictable
and the assertion readable. -/
private def refused (world : World) (index : Nat) (act : Action y) : List String :=
  let named := s!"refused{index}"
  let owed :=
    match refusal world act with
    | some code => variant code
    | none => "TheAdversaryOwesNoCodeHereWhichIsItsOwnBug"
  [s!"    let {named} = worker"]
    ++ handed 8 index act
    ++ [ "        .unwrap_err();"
       , s!"    assert_eq!(*{named}.code(), AxCode::{owed});"
       , "    // A refusal is a promise in three parts; a code with no way forward"
       , "    // keeps only one of them."
       , s!"    assert!(!{named}.recovery().is_empty());" ]

/-- Whether any step names an address, and so needs `Address`. -/
private def addressing (trace : Trace) : Bool :=
  trace.any fun packed =>
    match packed with
    | ⟨_, .raise ..⟩ => true
    | ⟨_, .work ..⟩ => true
    | ⟨_, .stop (.building _)⟩ => true
    | ⟨_, .resume (.building _)⟩ => true
    | _ => false

/-- Whether any step is expected to be refused, and so needs `AxCode`. -/
private def refusing (trace : Trace) : Bool :=
  (sequenced trace).any fun entry => !owed entry.snd.fst entry.snd.snd.act

/-- Whether any step reads the city back, and so needs the helper. -/
private def looking (trace : Trace) : Bool :=
  trace.any fun packed => match packed with | ⟨_, .look⟩ => true | _ => false

/-- Exactly the imports the emitted test uses.

Not one more: the workspace refuses an unused import, so a renderer that always
emitted the same header would produce a file nobody could commit. -/
private def imports (trace : Trace) : List String :=
  let wanted :=
    ((if addressing trace then ["Address"] else [])
      ++ (if refusing trace then ["AxCode"] else [])
      ++ ["IdemKey", "RunId", "Seq"]).toArray.qsort (· < ·) |>.toList
  let taken :=
    match wanted with
    | [only] => "use kernel::" ++ only ++ ";"
    | names => "use kernel::{" ++ String.intercalate ", " names ++ "};"
  [taken, "use sprawling::assembly;"]

/-- The helper a `look` step calls, emitted only when one is in the trace.

It reads the Ledger rather than the worker's own fields, because what a city is,
is what its history says. Joined rather than a list of its own: this is one
element of the outer join, and a trailing newline would end the file on a blank
line. -/
private def standingHelper : String :=
  String.intercalate "\n"
    [ "/// The addresses this city's history says stand, in the order it wrote them."
    , "///"
    , "/// The address is read from the payload rather than from the envelope: a"
    , "/// city-level record is written against `RunId::CITY` with no address of its"
    , "/// own, so an envelope check would pass for the wrong reason."
    , "fn standing(ledger: &std::path::Path) -> Vec<String> {"
    , "    let verified = runtime::replay::verify_ledger_dir(ledger).unwrap();"
    , "    let mut addresses: Vec<String> = verified"
    , "        .raw_lines()"
    , "        .iter()"
    , "        .filter_map(|line| {"
    , "            let record = kernel::EventRecord::parse_line(line).unwrap();"
    , "            if record.kind() != kernel::EventKind::BuildingCreated {"
    , "                return None;"
    , "            }"
    , "            record"
    , "                .data()"
    , "                .as_map()"
    , "                .get(\"addr\")"
    , "                .and_then(serde_json::Value::as_str)"
    , "                .map(str::to_owned)"
    , "        })"
    , "        .collect();"
    , "    addresses.sort();"
    , "    addresses"
    , "}" ]

private def preamble (name : String) (trace : Trace) : List String :=
  [ "// This Source Code Form is subject to the terms of the Mozilla Public"
  , "// License, v. 2.0. If a copy of the MPL was not distributed with this"
  , "// file, You can obtain one at https://mozilla.org/MPL/2.0/."
  , "// Copyright (c) 2026 2youg1 and the sprawling contributors"
  , ""
  , "//! A trace the adversary found, kept here so this repository remembers it."
  , "//!"
  , "//! Written by `adversary/src/Sprawling/Regression.lean` and compared against it"
  , "//! byte for byte. Change the trace there; changing it here turns the adversary"
  , "//! red, which is exactly what should happen when the two disagree."
  , "//!"
  , "//! The adversary drives the shipped binary over the wire. This enters by the"
  , "//! same door `channels::server` does, so the trace runs without a port."
  , ""
  , "#![allow("
  , "    clippy::unwrap_used,"
  , "    clippy::expect_used,"
  , "    clippy::panic,"
  , "    clippy::indexing_slicing,"
  , "    reason = \"test code\""
  , ")]"
  , "" ]
  ++ imports trace
  ++ [ ""
     , "#[test]"
     , "fn " ++ name ++ "() {"
     , "    let dir = tempfile::tempdir().unwrap();" ]
  ++ (if looking trace then
        ["    let raised = assembly::init_city(dir.path()).unwrap();"]
      else
        ["    assembly::init_city(dir.path()).unwrap();"])
  ++ [ ""
     , "    // The vault is the in-session one: a test that reached the platform"
     , "    // credential service would write to the machine running it."
     , "    let mut worker = assembly::RunWorker::new("
     , "        dir.path(),"
     , "        gateway::Custodian::in_memory(),"
     , "        runtime::diagnostics::Diagnostics::off(),"
     , "    )"
     , "    .unwrap();"
     , "" ]

private def closing (trace : Trace) : List String :=
  if looking trace then ["}", "", standingHelper] else ["}"]

/-- Renders a trace as a Rust integration test. -/
def render (name : String) (trace : Trace) : String :=
  String.join ((preamble name trace ++ walked (sequenced trace) ++ closing trace).map (· ++ "\n"))
where
  -- The blank line separates two steps, so the last step does not get one.
  -- rustfmt deletes an empty line before a closing brace, and a renderer that
  -- emitted one would produce a file `cargo fmt --check` rejects — which is to
  -- say a deliverable nobody can commit.
  walked : List (Nat × World × Any) → List String
    | [] => []
    | (index, world, packed) :: rest =>
      let act := packed.act
      (if owed world act then succeeded world index act else refused world index act)
        ++ (if rest.isEmpty then [] else [""])
        ++ walked rest

end Sprawling
