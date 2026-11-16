// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{HEADER, parse};

#[test]
fn task_spans_convert_milliseconds_and_count_model_invocations() {
    assert_eq!(
        parse(task_csv("1").as_bytes()).map(|rows| rows.len()),
        Ok(3)
    );
    let error = parse(task_csv("2").as_bytes()).unwrap_err();
    assert_eq!(error.code, "E_INVALID_INPUT");
    assert!(error.subject.contains("request count"));
}

fn task_csv(requests: &str) -> String {
    format!(
        "{}\n1,w,a,,0,1,r,b,arm,agent,task_wall,work,case,c,,,wall,ms,0,1000,none,,0,1,1,,fixture,\n1,n,a,,,,r,b,arm,agent,task_requests,work,case,c,,,requests,requests,0,{requests},none,,0,1,1,,fixture,\n1,m,a,r,0.2,0.6,r,b,arm,agent,request_latency,work,case,c,,,model_latency,ms,0,400,none,,0,1,1,,fixture,\n",
        HEADER.join(",")
    )
}
