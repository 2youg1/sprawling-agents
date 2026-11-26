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

use std::io::{self, ErrorKind};
use std::path::Path;
use std::time::Duration;

use axum::http::{Method, Request, StatusCode};
use bytes::Buf;
use kernel::{Address, AxCode, EventKind, IdemKey, RunId, Seq};
use remote_access::door::DeviceId;
use remote_access::handshake::{self, Invitation, PairReply, Reply, Session};
use remote_access::keys::{SigningKey, VerifyingKey};
use remote_access::seal::Payload;
use wtransport::endpoint::{ConnectOptions, endpoint_side::Client};
use wtransport::{ClientConfig, Connection, Endpoint, Identity, RecvStream, SendStream};

use super::harness::PATIENCE;

const WSAEACCES: i32 = 10_013;

fn tcp_port_unavailable(source: &io::Error) -> bool {
    source.kind() == ErrorKind::AddrInUse
        || (cfg!(windows) && source.raw_os_error() == Some(WSAEACCES))
}

#[cfg(windows)]
#[test]
fn a_winsock_bind_refusal_rejects_the_candidate_port() {
    assert!(tcp_port_unavailable(&io::Error::from_raw_os_error(
        WSAEACCES
    )));
    assert!(!tcp_port_unavailable(&io::Error::from(
        ErrorKind::PermissionDenied
    )));
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Visit {
    pub(crate) welcomed: bool,

    pub(crate) read_answered: bool,

    pub(crate) act_carried_out: bool,

    pub(crate) local_only_refused: Option<AxCode>,
}

async fn visit(gateway: &PublicDoor, invitation: &Invitation) -> Visit {
    let key = SigningKey::from_seed(&[9; 32]).unwrap();
    let client = gateway.client();
    let options = ConnectOptions::builder(format!("{}/remote/pair", gateway.origin))
        .add_header("origin", "https://another.example")
        .build();
    assert!(client.connect(options).await.is_err());
    let first = handshake::device_pair_hello([2; 32]).unwrap();
    let hello = first.hello().as_bytes();
    for (kind, announced, supplied) in [
        (0, hello.len(), 0),
        (0, hello.len(), 1),
        (0, hello.len(), hello.len() / 2),
        (0, hello.len(), hello.len().saturating_sub(1)),
        (0, 16 * 1024 + 1, 0),
        (u8::MAX, hello.len(), 0),
    ] {
        let mut socket = gateway.open(&client, "/remote/pair").await;
        let [a, b, c, d] = u32::try_from(announced).unwrap().to_be_bytes();
        socket.upload.write_all(&[kind, a, b, c, d]).await.unwrap();
        socket.upload.write_all(&hello[..supplied]).await.unwrap();
        socket.upload.finish().await.unwrap();
        assert_eq!(socket.packet().await, (1, b"E_WIRE_MISMATCH".to_vec()));
    }

    let (id, city) = pair(gateway, &client, invitation, &key).await;
    let (mut socket, mut session) = admitted(gateway, &client, id, &city, &key).await;
    let greeting = wire::ClientFrame::Hello(wire::Hello {
        wire_v: wire::WIRE_V,
        schema: wire::schema_hash(),
        token: None,
    });
    let asking = wire::ClientFrame::Ask(wire::Ask {
        ask_id: wire::AskId(1),
        query: wire::Query::Metrics,
    });
    let halting = wire::ClientFrame::Command(Box::new(wire::WireCommand::Halt {
        scope: wire::HaltScope::City,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"remote halt"),
    }));
    let revealing = wire::ClientFrame::Command(Box::new(wire::WireCommand::Reveal {
        at: Address::parse("lab").unwrap(),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"remote reveal"),
    }));
    let welcomed = exchange(&mut socket, &mut session, &greeting, |frame| {
        matches!(frame, wire::ServerFrame::Welcome(_))
    })
    .await;
    let read_answered = exchange(&mut socket, &mut session, &asking, |frame| {
        matches!(frame, wire::ServerFrame::Answered(_))
    })
    .await;
    let act_carried_out = exchange(
        &mut socket,
        &mut session,
        &halting,
        |frame| matches!(frame, wire::ServerFrame::Event(record) if record.kind() == EventKind::CityHalted),
    ).await;
    let local_only_refused = exchange(&mut socket, &mut session, &revealing, |frame| {
        matches!(frame, wire::ServerFrame::Refusal(_))
    })
    .await;
    Visit {
        welcomed: welcomed.is_some(),
        read_answered: read_answered.is_some(),
        act_carried_out: act_carried_out.is_some(),
        local_only_refused: match local_only_refused {
            Some(wire::ServerFrame::Refusal(refused)) => Some(*refused.code()),
            Some(_) | None => None,
        },
    }
}

async fn exchange(
    socket: &mut Duplex,
    session: &mut Session,
    frame: &wire::ClientFrame,
    wanted: fn(&wire::ServerFrame) -> bool,
) -> Option<wire::ServerFrame> {
    let text = serde_json::to_string(frame).unwrap();
    let sealed = session
        .sealer
        .seal(&Payload::Frame(text).to_bytes())
        .unwrap();

    socket.send(&sealed).await;
    loop {
        let sealed = socket.receive().await;
        let opened = session
            .opener
            .open(&sealed)
            .expect("open the city's sealed answer");
        let Payload::Frame(text) =
            Payload::from_bytes(&opened).expect("read the city's answer payload")
        else {
            continue;
        };
        let frame: wire::ServerFrame =
            serde_json::from_str(&text).expect("read the city's answer frame");
        if wanted(&frame) {
            return Some(frame);
        }
    }
}

async fn pair(
    gateway: &PublicDoor,
    client: &Endpoint<Client>,
    invitation: &Invitation,
    key: &SigningKey,
) -> (DeviceId, VerifyingKey) {
    let pairing = handshake::device_pair_hello([1; 32]).unwrap();
    let mut socket = gateway.open(client, "/remote/pair").await;
    socket.send(pairing.hello().as_bytes()).await;
    let reply = PairReply::from_bytes(&socket.receive().await).unwrap();
    let claimed = pairing.claim(&reply, invitation, key).unwrap();
    socket.send(&claimed.sealed).await;
    let mut session = claimed.session;
    let id = session.opener.open(&socket.receive().await).unwrap();
    assert_eq!(socket.packet().await, (1, Vec::new()));
    (
        DeviceId::from_entropy(<[u8; 16]>::try_from(id).unwrap()),
        claimed.city,
    )
}

async fn admitted(
    gateway: &PublicDoor,
    client: &Endpoint<Client>,
    id: DeviceId,
    city: &VerifyingKey,
    key: &SigningKey,
) -> (Duplex, Session) {
    let waiting = handshake::device_hello(id, [7; 32]).unwrap();
    let mut socket = gateway.open(client, "/remote/session").await;
    socket.send(waiting.hello().as_bytes()).await;
    let reply = Reply::from_bytes(&socket.receive().await).unwrap();
    let (finish, session) = waiting.finish(&reply, city, key).unwrap();
    socket.send(finish.as_bytes()).await;
    (socket, session)
}

struct Duplex {
    _connection: Connection,
    upload: SendStream,
    download: RecvStream,
}

impl Duplex {
    async fn send(&mut self, bytes: &[u8]) {
        let [a, b, c, d] = u32::try_from(bytes.len()).unwrap().to_be_bytes();
        self.upload.write_all(&[0, a, b, c, d]).await.unwrap();
        self.upload.write_all(bytes).await.unwrap();
    }

    async fn receive(&mut self) -> Vec<u8> {
        let (kind, body) = self.packet().await;
        assert_eq!(kind, 0, "the city stopped before sending the next frame");
        body
    }

    async fn packet(&mut self) -> (u8, Vec<u8>) {
        let mut header = [0_u8; 5];
        self.download.read_exact(&mut header).await.unwrap();
        let [kind, a, b, c, d] = header;
        let length = u32::from_be_bytes([a, b, c, d]);
        let mut body = vec![0_u8; length.try_into().unwrap()];
        self.download.read_exact(&mut body).await.unwrap();
        (kind, body)
    }
}

pub(crate) struct PublicDoor {
    address: std::net::SocketAddr,
    origin: String,
    identity: Identity,
}

impl PublicDoor {
    pub(crate) fn configure(city_root: &Path) -> Self {
        let file = kernel::layout::CityLayout::new(city_root).city_config();
        let before = match std::fs::read_to_string(&file) {
            Ok(before) => before,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(source) => panic!(
                "read the city test settings at {}: {source}",
                file.display()
            ),
        };
        let identity = Identity::self_signed(["127.0.0.1"]).unwrap();
        let certificate = file.with_file_name("door-certificate.pem");
        let key = file.with_file_name("door-key.pem");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            identity
                .certificate_chain()
                .store_pemfile(&certificate)
                .await
                .unwrap();
            identity
                .private_key()
                .store_secret_pemfile(&key)
                .await
                .unwrap();
        });
        let (udp, tcp) = (0..8)
            .find_map(|_| {
                let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
                match std::net::TcpListener::bind(udp.local_addr().unwrap()) {
                    Ok(tcp) => Some((udp, tcp)),
                    Err(source) if tcp_port_unavailable(&source) => None,
                    Err(source) => panic!("reserve the test port for TCP: {source}"),
                }
            })
            .expect("reserve one unused port for both TCP and UDP");
        let address = udp.local_addr().unwrap();
        let origin = format!("https://{address}");
        let table = format!(
            "
[remote]
route = 'h3'
url = '{origin}'
listen = '{address}'
certificate = '{}'
private_key = '{}'
",
            certificate.display(),
            key.display()
        );
        std::fs::write(&file, before + &table).unwrap();
        drop((udp, tcp));
        Self {
            address,
            origin,
            identity,
        }
    }

    fn certificate(&self) -> &wtransport::tls::Certificate {
        self.identity
            .certificate_chain()
            .as_slice()
            .first()
            .unwrap()
    }

    fn client(&self) -> Endpoint<Client> {
        Endpoint::client(
            ClientConfig::builder()
                .with_bind_default()
                .with_server_certificate_hashes([self.certificate().hash()])
                .build(),
        )
        .unwrap()
    }

    async fn open(&self, client: &Endpoint<Client>, path: &str) -> Duplex {
        let options = ConnectOptions::builder(format!("{}{path}", self.origin))
            .add_header("origin", &self.origin)
            .build();
        let connection = client.connect(options).await.unwrap();
        let (upload, download) = connection.open_bi().await.unwrap().await.unwrap();
        Duplex {
            _connection: connection,
            upload,
            download,
        }
    }

    pub(crate) fn visit(&self, invitation: &Invitation) -> Visit {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            tokio::time::timeout(PATIENCE, visit(self, invitation))
                .await
                .unwrap()
        })
    }

    pub(crate) fn page(&self) -> Vec<u8> {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let mut trust = rustls::RootCertStore::empty();
            trust.add(self.certificate().der().to_vec().into()).unwrap();
            let mut tls = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
                rustls::crypto::aws_lc_rs::default_provider(),
            ))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_root_certificates(trust)
            .with_no_client_auth();
            tls.alpn_protocols = vec![b"h3".to_vec()];
            let tls = quinn::crypto::rustls::QuicClientConfig::try_from(tls).unwrap();
            let mut client = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
            let mut config = quinn::ClientConfig::new(std::sync::Arc::new(tls));
            let mut transport = quinn::TransportConfig::default();
            transport.keep_alive_interval(Some(Duration::from_secs(1)));
            config.transport_config(std::sync::Arc::new(transport));
            client.set_default_client_config(config);
            let idle = client
                .connect(self.address, "127.0.0.1")
                .unwrap()
                .await
                .unwrap();
            let quic = client
                .connect(self.address, "127.0.0.1")
                .unwrap()
                .await
                .unwrap();
            let (mut h3, mut requests) = h3::client::new(h3_quinn::Connection::new(quic))
                .await
                .unwrap();
            let _drive = tokio::spawn(std::future::poll_fn(move |cx| h3.poll_close(cx)));
            for (method, url, status) in [
                (
                    Method::POST,
                    format!("{}/remote/frame", self.origin),
                    StatusCode::METHOD_NOT_ALLOWED,
                ),
                (
                    Method::GET,
                    "https://other.example/".to_owned(),
                    StatusCode::MISDIRECTED_REQUEST,
                ),
                (Method::GET, format!("{}/", self.origin), StatusCode::OK),
            ] {
                let mut stream = requests
                    .send_request(Request::builder().method(method).uri(url).body(()).unwrap())
                    .await
                    .unwrap();
                stream.finish().await.unwrap();
                assert_eq!(stream.recv_response().await.unwrap().status(), status);
                if status == StatusCode::OK {
                    let mut body = Vec::new();
                    while let Some(mut next) = stream.recv_data().await.unwrap() {
                        body.extend_from_slice(&next.copy_to_bytes(next.remaining()));
                    }
                    tokio::time::timeout(Duration::from_secs(15), idle.closed())
                        .await
                        .expect("the city releases a connection that sends no H3 request");
                    return body;
                }
            }
            panic!("the public page did not answer")
        })
    }
}
