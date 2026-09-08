// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use std::collections::BTreeSet;

#[test]
fn lower_layer_overrides_upper() {
    let ladder = LayeredValue {
        city: Some(ClockStampGranularity::Hour),
        building: Some(ClockStampGranularity::Minute),
        resident: None,
    };
    assert_eq!(
        freeze(
            &ladder,
            &LayeredValue::default(),
            &LayeredValue::default(),
            &LayeredValue::default(),
            &LayeredValue::default()
        )
        .clock_stamp,
        ClockStampGranularity::Minute
    );
    let ladder = LayeredValue {
        city: Some(ClockStampGranularity::Hour),
        building: Some(ClockStampGranularity::Minute),
        resident: Some(ClockStampGranularity::FiveMinute),
    };
    assert_eq!(
        freeze(
            &ladder,
            &LayeredValue::default(),
            &LayeredValue::default(),
            &LayeredValue::default(),
            &LayeredValue::default()
        )
        .clock_stamp,
        ClockStampGranularity::FiveMinute
    );
}

#[test]
fn absence_everywhere_takes_the_policy_default() {
    let ladder: LayeredValue<ClockStampGranularity> = LayeredValue::default();
    let frozen = freeze(
        &ladder,
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
    );
    assert_eq!(frozen.clock_stamp, CLOCK_STAMP_DEFAULT);
    assert_eq!(frozen.clock_stamp, ClockStampGranularity::Off);
    assert!(frozen.clock_zones.is_empty());
    assert_eq!(
        frozen.effort, None,
        "an unstated effort stays unstated: the provider's default is not ours to name"
    );
}

#[test]
fn effort_resolves_down_the_same_ladder() {
    let effort = LayeredValue {
        city: Some(Effort::Low),
        building: Some(Effort::Max),
        resident: None,
    };
    let frozen = freeze(
        &LayeredValue::default(),
        &LayeredValue::default(),
        &effort,
        &LayeredValue::default(),
        &LayeredValue::default(),
    );
    assert_eq!(frozen.effort, Some(Effort::Max));
}

#[test]
fn the_sandbox_resolves_as_one_value_so_a_thin_layer_only_narrows() {
    let permissive = SandboxLimits {
        shell: true,
        fuel: 10,
        mounts: vec![Address::parse("lab/docs").unwrap()],
        env_passthrough: vec![EnvVarName::parse("ProgramFiles").unwrap()],
    };
    let terse = SandboxLimits {
        fuel: 20,
        ..SandboxLimits::default()
    };
    let ladder = LayeredValue {
        city: Some(permissive),
        building: Some(terse),
        resident: None,
    };
    let frozen = freeze(
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
        &ladder,
        &LayeredValue::default(),
    );
    assert_eq!(frozen.sandbox.fuel, 20);
    assert!(
        !frozen.sandbox.shell,
        "a layer that speaks about the sandbox speaks about all of it, and silence is the              closed answer"
    );
    assert!(frozen.sandbox.mounts.is_empty());
}

#[test]
fn an_unstated_sandbox_is_closed_with_the_default_fuel() {
    let frozen = freeze(
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
    );
    assert!(!frozen.sandbox.shell);
    assert_eq!(
        frozen.sandbox.fuel,
        crate::consts_policy::SANDBOX_FUEL_DEFAULT
    );
    assert!(frozen.sandbox.mounts.is_empty());
}

#[test]
fn zones_override_as_a_whole_list() {
    let zones = LayeredValue {
        city: Some(vec![
            ClockZone {
                id: "tokyo".to_owned(),
                offset_min: 540,
            },
            ClockZone {
                id: "berlin".to_owned(),
                offset_min: 120,
            },
        ]),
        building: Some(vec![ClockZone {
            id: "nyc".to_owned(),
            offset_min: -240,
        }]),
        resident: None,
    };
    let frozen = freeze(
        &LayeredValue::default(),
        &zones,
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
    );
    assert_eq!(frozen.clock_zones.len(), 1);
    assert_eq!(frozen.clock_zones[0].id, "nyc");
}

#[test]
fn servers_override_as_a_whole_table_and_silence_reaches_nothing() {
    let server = |raw: &str| McpServer {
        label: ServerLabel::parse(raw).unwrap(),
        transport: McpTransport::Stdio {
            command: "mcp-server".to_owned(),
            args: Vec::new(),
        },
    };
    let ladder = LayeredValue {
        city: Some(vec![server("apps"), server("mail")]),
        building: Some(vec![server("apps")]),
        resident: None,
    };
    let frozen = freeze(
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
        &ladder,
    );
    assert_eq!(frozen.mcp.len(), 1);
    assert_eq!(frozen.mcp[0].label.as_str(), "apps");

    let silent = freeze(
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
    );
    assert!(
        silent.mcp.is_empty(),
        "a building nobody granted a server reaches none of them"
    );
}

#[test]
fn frozen_and_live_share_no_field() {
    let frozen = serde_json::to_value(FrozenConfig {
        clock_stamp: ClockStampGranularity::Off,
        clock_zones: Vec::new(),
        sandbox: SandboxLimits::default(),
        effort: None,
        mcp: Vec::new(),
    })
    .unwrap();
    let live = serde_json::to_value(LiveConfig {}).unwrap();
    let frozen_keys: BTreeSet<String> = frozen
        .as_object()
        .expect("frozen serializes to an object")
        .keys()
        .cloned()
        .collect();
    let live_keys: BTreeSet<String> = live
        .as_object()
        .expect("live serializes to an object")
        .keys()
        .cloned()
        .collect();
    assert!(
        frozen_keys.is_disjoint(&live_keys),
        "freeze line breached: {frozen_keys:?} vs {live_keys:?}"
    );
}

#[test]
fn granularity_serde_is_snake_case() {
    let json = serde_json::to_string(&ClockStampGranularity::FiveMinute).unwrap();
    assert_eq!(json, "\"five_minute\"");
}

#[test]
fn a_credential_shaped_variable_name_is_refused_where_the_config_is_read() {
    // The judgement has to happen here: a name handed to a child process
    // cannot be taken back, so it is refused at the one construction
    // point rather than at the point it would have been used.
    for shaped in [
        "AWS_SECRET_ACCESS_KEY",
        "GITHUB_TOKEN",
        "OPENAI_API_KEY",
        "npm_password",
        "SESSION_COOKIE",
    ] {
        let err = EnvVarName::parse(shaped).expect_err(shaped);
        assert_eq!(*err.code(), crate::AxCode::InvalidArgs, "{shaped}");
        assert!(!err.recovery().is_empty(), "{shaped}");
    }
    for spelled in ["", "HAS=EQUALS", "HAS\u{7}CONTROL"] {
        assert!(EnvVarName::parse(spelled).is_err(), "{spelled:?}");
    }
    for plain in ["PATH", "ProgramFiles(x86)", "VSINSTALLDIR", "CARGO_HOME"] {
        assert_eq!(
            EnvVarName::parse(plain).expect(plain).as_str(),
            plain,
            "a name that names no credential is admitted as written"
        );
    }
}

#[test]
fn the_declared_environment_names_ride_the_same_whole_value_ladder_as_mounts() {
    let city_layer = SandboxLimits {
        env_passthrough: vec![EnvVarName::parse("CARGO_HOME").unwrap()],
        ..SandboxLimits::default()
    };
    let ladder = LayeredValue {
        city: Some(city_layer),
        building: Some(SandboxLimits::default()),
        resident: None,
    };
    let frozen = freeze(
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue::default(),
        &ladder,
        &LayeredValue::default(),
    );
    assert!(
        frozen.sandbox.env_passthrough.is_empty(),
        "a layer that speaks about the sandbox speaks about all of it"
    );
}
