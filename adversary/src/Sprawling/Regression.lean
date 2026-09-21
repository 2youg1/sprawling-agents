-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Model
import Sprawling.Provider

/-!
# What this adversary delivers: Rust tests.

A counterexample that stays in Lean is knowledge this repository does not have.
Rendering it as a test beside the code it accuses moves the knowledge into the
language that ships, and leaves nothing here that could grow into a second
authority.

Both worlds deliver here. `Sprawling.Model` hands over a trace, whose steps and
polarity are read off the model; `Sprawling.Provider` hands over the two facts
its own checks found broken — one endpoint written down five ways, and a model
registered with no output ceiling — as the pair of steps a person walks to reach
them. Nothing is restated: the spellings, the endpoint's name and the model id
are read from the module that owns them, so a cast changed there is a file
rendered differently here.

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
    [ "/// The addresses this city's history says stand, sorted."
    , "///"
    , "/// The address is read from the payload rather than from the envelope: a"
    , "/// city-level record is written against `RunId::CITY` with no address of its"
    , "/// own, so an envelope check would pass for the wrong reason."
    , "fn standing(ledger: &std::path::Path) -> Vec<String> {"
    , "    let mut addresses: Vec<String> = stated(ledger, kernel::EventKind::BuildingCreated, \"addr\")"
    , "        .iter()"
    , "        .filter_map(|addr| addr.as_str().map(str::to_owned))"
    , "        .collect();"
    , "    addresses.sort();"
    , "    addresses"
    , "}" ]

/-- A city raised, and the worker every test here drives.

`named` says whether the test reads the city's history back afterwards. A name
nothing reads is a warning, and this workspace refuses one. -/
private def cityAndWorker (named : Bool) : List String :=
  [ "    let dir = tempfile::tempdir().unwrap();"
  , if named then "    let raised = assembly::init_city(dir.path()).unwrap();"
    else "    assembly::init_city(dir.path()).unwrap();"
  , ""
  , "    // The vault is the in-session one: a test that reached the platform"
  , "    // credential service would write to the machine running it."
  , "    let mut worker = assembly::RunWorker::new("
  , "        dir.path(),"
  , "        gateway::Custodian::in_memory(),"
  , "        runtime::diagnostics::Diagnostics::off(),"
  , "    )"
  , "    .unwrap();"
  , "" ]

private def preamble (name : String) (trace : Trace) : List String :=
  [ "// This Source Code Form is subject to the terms of the Mozilla Public"
  , "// License, v. 2.0. If a copy of the MPL was not distributed with this"
  , "// file, You can obtain one at https://mozilla.org/MPL/2.0/."
  , "// Copyright (c) 2026 2youg1 and the sprawling contributors"
  , ""
  , "//! The traces the adversary found, kept here so this repository remembers them."
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
     , "fn " ++ name ++ "() {" ]
  ++ cityAndWorker (looking trace)

/-- One spelling of one endpoint, entered under a name of its own.

The line is written whole rather than wrapped, which holds while the longest
spelling keeps it inside the width rustfmt is configured for; a longer
authority than this one would be reformatted, and the comparison in
`adversary/test/Main.lean` would report that as a drift. -/
private def attaching (spelling : String) (index : Nat) : String :=
  s!"    attach(&mut worker, {quoted s!"{relayName}-{index}"}, {quoted spelling}).unwrap();"

/-- Every spelling of one endpoint reaches one registration.

The defect this remembers is recorded in `adversary-SPEC.md` section 4, fourth
finding: the normalisation existed, had its own tests, and had no caller, so the
city wrote down five different URLs for one endpoint and the first call to four
of them answered 404. -/
private def oneUrlRegistered : List String :=
  let spelled := spellings deadAuthority
  [ "#[test]"
  , "fn every_spelling_of_one_endpoint_is_registered_as_one_url() {" ]
  ++ cityAndWorker true
  ++ [ "    // A provider prints one endpoint several ways and a person pastes"
     , "    // whichever one they were shown. Nothing listens on this address, so"
     , "    // every attachment below is one the person's own model id carried." ]
  ++ spelled.zipIdx.map (fun entry => attaching entry.fst entry.snd)
  ++ [ ""
     , "    let written = stated("
     , "        &raised.ledger_dir,"
     , "        kernel::EventKind::EndpointAttached,"
     , "        \"base_url\","
     , "    );"
     , "    // One registration per spelling, and the same URL in each: what is"
     , "    // asserted is the relation between the readings, so no normalisation"
     , "    // is recomputed here."
     , s!"    assert_eq!(written.len(), {spelled.length});"
     , "    assert!("
     , "        written.windows(2).all(|pair| pair[0] == pair[1]),"
     , "        \"one endpoint was written down as {written:?}\""
     , "    );"
     , "}" ]

/-- A model no catalogue prices is registered with a ceiling anyway.

The defect this remembers is the fifth finding of the same section: the ladder
in `gateway::provider::ceiling` answered for every rung nobody filled in, and
`select_model` walked a chain of its own that stopped one rung short — so the
Anthropic wire, which requires `max_tokens` in every request, could not write
one for a model the person had just picked from a list. -/
private def ceilingRegistered : List String :=
  [ "#[test]"
  , "fn a_model_no_catalogue_prices_is_registered_with_a_ceiling() {" ]
  ++ cityAndWorker true
  ++ [ s!"    attach(&mut worker, {quoted relayName}, {quoted attachedUrl}).unwrap();"
     , "    // The box a person left empty. The Anthropic wire requires"
     , "    // `max_tokens` in every request, so a registration that kept the"
     , "    // ceiling unstated is a model this city cannot call at all."
     , "    worker"
     , "        .handle(channels::Command::SelectModel {"
     , s!"            endpoint: channels::ProviderName::parse({quoted relayName}).unwrap(),"
     , s!"            model: {quoted unlistedModel}.to_owned(),"
     , "            tag: kernel::ModelTag::Main,"
     , "            context_tokens: 0,"
     , "            max_output_tokens: None,"
     , "            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b\"select\"),"
     , "        })"
     , "        .unwrap();"
     , ""
     , "    let ceilings = stated("
     , "        &raised.ledger_dir,"
     , "        kernel::EventKind::ModelSelected,"
     , "        \"max_output_tokens\","
     , "    );"
     , "    assert!("
     , "        ceilings"
     , "            .first()"
     , "            .and_then(serde_json::Value::as_u64)"
     , "            .is_some_and(|tokens| tokens > 0),"
     , "        \"a model nobody priced was registered with {ceilings:?}\""
     , "    );"
     , "    // Which rung answered, read back in the ladder's own spelling: this"
     , "    // id is in no catalogue and under no preset host, so the policy"
     , "    // default is the rung that was left."
     , "    let rungs = stated("
     , "        &raised.ledger_dir,"
     , "        kernel::EventKind::ModelSelected,"
     , "        \"ceiling_from\","
     , "    );"
     , "    assert_eq!("
     , "        rungs.first().and_then(serde_json::Value::as_str),"
     , "        Some(gateway::CeilingSource::Policy.as_str())"
     , "    );"
     , "}" ]

/-- The attachment both provider tests make, written once.

The model id is named rather than left to the probe, for the product's own
reason: an attachment that declared no id is refused when the probe reaches
nobody, and nothing listens on this address. -/
private def attachHelper : List String :=
  [ "/// One endpoint attached under a name of its own, with the model id the"
  , "/// person named."
  , "///"
  , "/// The id is named rather than left to the probe: most compatible"
  , "/// endpoints serve no model list, and this address serves nothing at"
  , "/// all, so an attachment that declared no id would be refused for an"
  , "/// interface the endpoint never promised."
  , "fn attach("
  , "    worker: &mut assembly::RunWorker,"
  , "    name: &str,"
  , "    base_url: &str,"
  , ") -> Result<(), kernel::AxError> {"
  , "    worker.handle(channels::Command::AttachEndpoint {"
  , "        name: channels::ProviderName::parse(name)?,"
  , "        base_url: base_url.to_owned(),"
  , "        dialect: kernel::DialectKind::Anthropic,"
  , "        secret: None,"
  , "        auth_header: None,"
  , s!"        admit: vec![{quoted unlistedModel}.to_owned()],"
  , "        tuning: channels::EndpointTuning::default(),"
  , "        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, name.as_bytes()),"
  , "    })"
  , "}" ]

/-- The one reader of the history in the emitted file.

Every assertion above asks it, and `standing` is written in terms of it rather
than beside it: two readers of one ledger would be two accounts of what a record
is. -/
private def statedHelper : List String :=
  [ "/// What this city's history states under one kind of record and one"
  , "/// field, oldest first."
  , "///"
  , "/// The ledger is the only reading these tests take: what a city"
  , "/// registered is what its history says it registered, and a field read"
  , "/// off a live structure would be a second account of the same fact."
  , "fn stated("
  , "    ledger: &std::path::Path,"
  , "    kind: kernel::EventKind,"
  , "    field: &str,"
  , ") -> Vec<serde_json::Value> {"
  , "    let verified = runtime::replay::verify_ledger_dir(ledger).unwrap();"
  , "    verified"
  , "        .raw_lines()"
  , "        .iter()"
  , "        .filter_map(|line| {"
  , "            let record = kernel::EventRecord::parse_line(line).unwrap();"
  , "            if record.kind() != kind {"
  , "                return None;"
  , "            }"
  , "            record.data().as_map().get(field).cloned()"
  , "        })"
  , "        .collect()"
  , "}" ]

/-- What follows the trace's own test: the two provider counterexamples, and
the helpers all three tests call. -/
private def closing (trace : Trace) : List String :=
  ["}", ""] ++ oneUrlRegistered ++ [""] ++ ceilingRegistered ++ [""] ++ attachHelper
    ++ [""] ++ statedHelper
    ++ (if looking trace then ["", standingHelper] else [])

/-- Renders what both worlds found as one Rust integration test file. -/
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
