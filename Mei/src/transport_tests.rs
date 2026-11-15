// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

    use std::io::{BufRead, BufReader};
    use std::thread;

    use super::*;

    fn server(response: &'static str) -> (String, thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut request = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.strip_prefix("Content-Length: ") {
                    length = value.trim().parse::<usize>().unwrap();
                }
                request.push_str(&line);
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            request.push_str(&String::from_utf8_lossy(&body));
            let mut stream = reader.into_inner();
            stream.write_all(response.as_bytes()).unwrap();
            stream.shutdown(std::net::Shutdown::Both).unwrap();
            request
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn a_post_writes_one_non_streaming_request_and_reads_json_to_eof() {
        let (base, handle) =
            server("HTTP/1.0 200 OK\r\nContent-Length: 25\r\n\r\n{\"stop_reason\":\"end_turn\"}");
        let client = Client::new(&base, "magpie".to_owned()).unwrap();
        let mut pulse = Vec::new();
        let answer = client
            .post(
                &serde_json::json!({"model": "测试", "messages": []}),
                &mut pulse,
            )
            .unwrap();
        assert_eq!(answer["stop_reason"], "end_turn");
        assert!(pulse.is_empty(), "{}", String::from_utf8_lossy(&pulse));
        let request = handle.join().unwrap();
        assert!(request.starts_with("POST /v1/messages HTTP/1.0\r\n"));
        assert!(request.contains("Content-Length: 32\r\n"));
        assert!(request.contains("x-api-key: magpie\r\n"));
        assert!(request.contains("anthropic-version: 2023-06-01\r\n"));
        assert!(!request.contains("Accept-Encoding"));
        let body = request.get(request.find('{').unwrap()..).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!({"model": "测试", "messages": []})
        );
    }

    #[test]
    fn a_non_200_returns_the_body_without_wrapping_it() {
        let (base, handle) = server("HTTP/1.0 429 Slow down\r\n\r\nrate limited");
        let client = Client::new(&base, "magpie".to_owned()).unwrap();
        let mut pulse = Vec::new();
        assert_eq!(
            client.post(&serde_json::json!({}), &mut pulse),
            Err(Error::Body("rate limited".to_owned()))
        );
        handle.join().unwrap();
    }

    #[test]
    fn malformed_boundaries_and_chunking_are_named() {
        let (base, handle) = server("not an HTTP response");
        let client = Client::new(&base, "magpie".to_owned()).unwrap();
        let mut pulse = Vec::new();
        let error = client
            .post(&serde_json::json!({}), &mut pulse)
            .unwrap_err()
            .to_string();
        assert!(error.contains("boundary") && error.contains("not an HTTP response"));
        handle.join().unwrap();
        let (base, handle) =
            server("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n");
        let client = Client::new(&base, "magpie".to_owned()).unwrap();
        let mut pulse = Vec::new();
        assert!(
            client
                .post(&serde_json::json!({}), &mut pulse)
                .unwrap_err()
                .to_string()
                .contains("Transfer-Encoding")
        );
        handle.join().unwrap();
    }

    #[test]
    fn url_and_key_validation_is_fail_closed() {
        for base in [
            "https://localhost:3425",
            "http://localhost:3425/v1",
            "http://user@localhost",
            "http://localhost?q=1",
            "http://",
        ] {
            assert!(Client::new(base, "magpie".to_owned()).is_err(), "{base}");
        }
        assert!(Client::new("http://127.0.0.1:3425/", "magpie".to_owned()).is_ok());
        assert!(Client::new("http://127.0.0.1:3425", "bad\nkey".to_owned()).is_err());
    }

    #[test]
    fn a_gateway_that_accepts_and_stalls_returns_instead_of_hanging() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stalled = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut byte = [0_u8; 1];
            // Never answer; hold the connection until the client gives up.
            while let Ok(count) = stream.read(&mut byte) {
                if count == 0 {
                    break;
                }
            }
        });
        let client = Client::with_timeouts(
            &format!("http://{address}"),
            "magpie".to_owned(),
            Timeouts {
                connect: Duration::from_secs(2),
                io: Duration::from_millis(200),
                pulse: Duration::from_millis(50),
            },
        )
        .unwrap();
        let mut pulse = Vec::new();
        let error = client
            .post(&serde_json::json!({}), &mut pulse)
            .unwrap_err()
            .to_string();
        assert!(error.contains("E_MEI_TRANSPORT"), "{error}");
        assert!(error.contains("read the Messages response"), "{error}");
        stalled.join().unwrap();
    }

    #[test]
    fn a_slow_gateway_emits_pulse_observations() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let delayed = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut length = 0;
            {
                let mut reader = BufReader::new(&mut stream);
                let mut line = String::new();
                loop {
                    line.clear();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("Content-Length: ") {
                        length = value.trim().parse::<usize>().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
            }
            thread::sleep(Duration::from_millis(150));
            stream
                .write_all(
                    b"HTTP/1.0 200 OK\r\n\r\n{\"stop_reason\":\"end_turn\",\"content\":\"ok\"}",
                )
                .unwrap();
        });
        let client = Client::with_timeouts(
            &format!("http://{address}"),
            "magpie".to_owned(),
            Timeouts {
                connect: Duration::from_secs(2),
                io: Duration::from_secs(2),
                pulse: Duration::from_millis(50),
            },
        )
        .unwrap();
        let mut pulse = Vec::new();
        let answer = client
            .post(
                &serde_json::json!({"model": "m", "messages": []}),
                &mut pulse,
            )
            .unwrap();
        assert_eq!(answer["stop_reason"], "end_turn");
        let lines: Vec<Value> = String::from_utf8(pulse)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(
            lines
                .iter()
                .any(|line| line == &json!({"type": "mei.obs", "kind": "pulse"})),
            "{lines:?}"
        );
        delayed.join().unwrap();
    }
