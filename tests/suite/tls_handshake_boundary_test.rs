//! Exercise the real rustls record parser used by WFL's TLS dependency graph.
//! A complete plaintext EncryptedExtensions must not share ServerHello's record.

use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use rustls::pki_types::{PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConfig, ServerConnection};

fn connections() -> (ClientConnection, ServerConnection) {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert = certified.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server = ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert.clone()], key.into())
        .unwrap();
    let mut roots = RootCertStore::empty();
    roots.add(cert).unwrap();
    let client = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    (
        ClientConnection::new(Arc::new(client), ServerName::try_from("localhost").unwrap())
            .unwrap(),
        ServerConnection::new(Arc::new(server)).unwrap(),
    )
}

fn server_flight(client: &mut ClientConnection, server: &mut ServerConnection) -> Vec<u8> {
    let mut hello = Vec::new();
    client.write_tls(&mut hello).unwrap();
    server.read_tls(&mut Cursor::new(hello)).unwrap();
    server.process_new_packets().unwrap();
    let mut flight = Vec::new();
    server.write_tls(&mut flight).unwrap();
    flight
}

fn coalesced_server_hello(flight: &[u8]) -> Vec<u8> {
    assert_eq!(flight[0], 22, "first record must be a handshake");
    assert_eq!(flight[5], 2, "first handshake must be ServerHello");
    let record_len = usize::from(u16::from_be_bytes([flight[3], flight[4]]));
    let mut record = flight[..5 + record_len].to_vec();
    // EncryptedExtensions with an empty extension vector: a complete, valid
    // handshake message, but deliberately sent before encryption takes effect.
    let extensions = [8, 0, 0, 2, 0, 0];
    record.extend_from_slice(&extensions);
    let new_len = u16::try_from(record_len + extensions.len()).unwrap();
    record[3..5].copy_from_slice(&new_len.to_be_bytes());
    record
}

#[test]
fn rejects_plaintext_encrypted_extensions_coalesced_with_server_hello() {
    let (mut client, mut server) = connections();
    let record = coalesced_server_hello(&server_flight(&mut client, &mut server));
    client.read_tls(&mut Cursor::new(record)).unwrap();
    let error = client
        .process_new_packets()
        .expect_err("must reject a plaintext message crossing the ServerHello key change");
    assert!(
        matches!(
            error,
            rustls::Error::PeerMisbehaved(rustls::PeerMisbehaved::KeyEpochWithPendingFragment)
        ),
        "{error:?}"
    );
    assert!(client.is_handshaking());
    let mut plaintext = [0; 1];
    assert!(client.reader().read(&mut plaintext).is_err());
}

#[test]
fn rejects_coalesced_record_at_every_transport_split() {
    let (mut client, mut server) = connections();
    let record_len = coalesced_server_hello(&server_flight(&mut client, &mut server)).len();
    // Exhaust the two-chunk transport partitions of the generated regression
    // record. Neither a split record header nor split handshake data may bypass
    // the encryption-level check. No timing or random seed controls this loop.
    for split in 1..record_len {
        let (mut client, mut server) = connections();
        let record = coalesced_server_hello(&server_flight(&mut client, &mut server));
        client.read_tls(&mut Cursor::new(&record[..split])).unwrap();
        client.process_new_packets().unwrap();
        client.read_tls(&mut Cursor::new(&record[split..])).unwrap();
        assert!(
            matches!(
                client.process_new_packets(),
                Err(rustls::Error::PeerMisbehaved(
                    rustls::PeerMisbehaved::KeyEpochWithPendingFragment
                ))
            ),
            "record split at byte {split}"
        );
    }
}

#[tokio::test]
async fn wfl_https_rejects_coalesced_record_over_a_real_socket() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    async fn read_record(socket: &mut TcpStream) -> Vec<u8> {
        let mut header = [0; 5];
        socket.read_exact(&mut header).await.unwrap();
        let length = usize::from(u16::from_be_bytes([header[3], header[4]]));
        let mut record = header.to_vec();
        record.resize(5 + length, 0);
        socket.read_exact(&mut record[5..]).await.unwrap();
        record
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (_, mut server) = connections();
    let peer = async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let hello = read_record(&mut socket).await;
        server.read_tls(&mut Cursor::new(hello)).unwrap();
        server.process_new_packets().unwrap();
        let mut flight = Vec::new();
        server.write_tls(&mut flight).unwrap();
        socket
            .write_all(&coalesced_server_hello(&flight))
            .await
            .unwrap();
        let alert = read_record(&mut socket).await;
        // Fatal unexpected_message, before certificate verification or any
        // application data. An EOF or timeout alone would not prove the fix.
        assert_eq!(alert[0], 21, "expected a TLS alert, got {alert:?}");
        assert_eq!(&alert[5..], &[2, 10]);
    };
    let request = async {
        let code = format!(
            r#"open url at "https://{address}/" with method "POST" and body "private" and read response as resp"#
        );
        let tokens = wfl::lexer::lex_wfl_with_positions(&code);
        let ast = wfl::parser::Parser::new(&tokens).parse().unwrap();
        let mut interpreter = wfl::Interpreter::new();
        let errors = interpreter
            .interpret(&ast)
            .await
            .expect_err("TLS must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("Failed to send HTTP POST request")),
            "{errors:?}"
        );
        assert!(interpreter.global_env().borrow().get("resp").is_none());
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(peer, request)
    })
    .await
    .expect("TLS rejection must finish promptly");
}

#[test]
fn accepts_correctly_framed_tls13_and_authenticated_application_data() {
    let (mut client, mut server) = connections();
    let flight = server_flight(&mut client, &mut server);
    client.read_tls(&mut Cursor::new(flight)).unwrap();
    client.process_new_packets().unwrap();
    let mut finished = Vec::new();
    client.write_tls(&mut finished).unwrap();
    server.read_tls(&mut Cursor::new(finished)).unwrap();
    server.process_new_packets().unwrap();
    assert!(!client.is_handshaking());
    assert!(!server.is_handshaking());
    server
        .writer()
        .write_all(b"authenticated response")
        .unwrap();
    let mut response = Vec::new();
    server.write_tls(&mut response).unwrap();
    client.read_tls(&mut Cursor::new(response)).unwrap();
    client.process_new_packets().unwrap();
    let mut plaintext = [0; 22];
    client.reader().read_exact(&mut plaintext).unwrap();
    assert_eq!(&plaintext, b"authenticated response");
}
