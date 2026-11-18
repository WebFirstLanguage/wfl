//! A real DNS server for the ordered-resolution tests.
//!
//! This is deliberately *not* a mock of WFL's resolver: it is a separate DNS
//! server that listens on a real UDP socket and a real TCP listener on
//! `127.0.0.1`, speaks the RFC 1035 wire format, and answers according to a
//! per-test script. The resolver under test talks to it exactly as it would
//! talk to `10.3.0.100`, so these tests exercise the real boundary
//! (`testing.md` §8.4: a test must not mock the boundary it verifies).
//!
//! Every question the server receives is logged with its transport, so a test
//! can assert *which* servers were asked, in *which* order, and — just as
//! importantly — which servers were never asked at all.

#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};

pub const TYPE_A: u16 = 1;
pub const TYPE_AAAA: u16 = 28;

pub const RCODE_FORMERR: u8 = 1;
pub const RCODE_SERVFAIL: u8 = 2;
pub const RCODE_NXDOMAIN: u8 = 3;
pub const RCODE_NOTIMP: u8 = 4;
pub const RCODE_REFUSED: u8 = 5;

/// How the server answers one question.
#[derive(Clone, Debug)]
pub enum Reply {
    /// NOERROR with every address of the asked-for family. If none of the
    /// addresses match the question's type, this is an empty NOERROR.
    Addresses(Vec<IpAddr>),
    /// NXDOMAIN ("no such name"), with an SOA record carrying the negative TTL.
    NxDomain,
    /// NOERROR with no answer records ("no data"), with an SOA record.
    NoData,
    /// A bare response code with no records (SERVFAIL, REFUSED, ...).
    Rcode(u8),
    /// Never answer.
    Silent,
    /// Answer with bytes that are not a DNS message.
    Garbage,
    /// Over UDP: an empty response with the TC (truncated) bit set.
    /// Over TCP: the full answer.
    TruncatedThenTcp(Vec<IpAddr>),
    /// First send a well-formed answer with the WRONG query ID pointing at
    /// `decoy`, then (after a short pause) the real reply.
    WrongIdThen { decoy: IpAddr, then: Box<Reply> },
    /// Wait, then send the inner reply.
    Delayed(Duration, Box<Reply>),
}

impl Reply {
    pub fn v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        Reply::Addresses(vec![IpAddr::V4(Ipv4Addr::new(a, b, c, d))])
    }

    pub fn loopback() -> Self {
        Self::v4(127, 0, 0, 1)
    }

    pub fn delayed(millis: u64, inner: Reply) -> Self {
        Reply::Delayed(Duration::from_millis(millis), Box::new(inner))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    Udp,
    Tcp,
}

/// One question as the server saw it.
#[derive(Clone, Debug)]
pub struct Query {
    /// Lowercased, without the trailing dot.
    pub name: String,
    pub qtype: u16,
    pub transport: Transport,
    pub at: Instant,
}

type Handler = dyn Fn(&Query) -> Reply + Send + Sync;

struct Shared {
    handler: Box<Handler>,
    log: Mutex<Vec<Query>>,
    ttl: u32,
}

/// A running fake DNS server. Dropping it stops both listeners.
pub struct FakeDns {
    pub addr: SocketAddr,
    shared: Arc<Shared>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for FakeDns {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

impl FakeDns {
    /// Start a server whose answers come from `handler`. Answer TTLs (and the
    /// SOA negative TTL) are 60 seconds.
    pub async fn start(handler: impl Fn(&Query) -> Reply + Send + Sync + 'static) -> Self {
        Self::start_with_ttl(60, handler).await
    }

    pub async fn start_with_ttl(
        ttl: u32,
        handler: impl Fn(&Query) -> Reply + Send + Sync + 'static,
    ) -> Self {
        // UDP and TCP must share one port, because a DNS server is addressed
        // by a single `ip:port` and the client falls back from UDP to TCP on
        // that same port. Bind UDP on an ephemeral port, then claim the same
        // TCP port; retry in the rare case another process holds it.
        let (udp, tcp) = {
            let mut bound = None;
            for _ in 0..50 {
                let udp = UdpSocket::bind("127.0.0.1:0").await.expect("bind udp");
                let port = udp.local_addr().unwrap().port();
                if let Ok(tcp) = TcpListener::bind(("127.0.0.1", port)).await {
                    bound = Some((udp, tcp));
                    break;
                }
            }
            bound.expect("could not bind UDP and TCP on the same port")
        };
        let addr = udp.local_addr().unwrap();
        let shared = Arc::new(Shared {
            handler: Box::new(handler),
            log: Mutex::new(Vec::new()),
            ttl,
        });

        let udp = Arc::new(udp);
        let udp_task = {
            let shared = Arc::clone(&shared);
            let udp = Arc::clone(&udp);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                loop {
                    let Ok((len, peer)) = udp.recv_from(&mut buf).await else {
                        continue;
                    };
                    let packet = buf[..len].to_vec();
                    let shared = Arc::clone(&shared);
                    let udp = Arc::clone(&udp);
                    // One task per question, so a Delayed or Silent answer for
                    // one question never holds up another.
                    tokio::spawn(async move {
                        let Some(parsed) = ParsedQuery::parse(&packet) else {
                            return;
                        };
                        let query = shared.record(&parsed, Transport::Udp);
                        let reply = (shared.handler)(&query);
                        for datagram in respond(&parsed, &reply, Transport::Udp, shared.ttl).await {
                            let _ = udp.send_to(&datagram, peer).await;
                        }
                    });
                }
            })
        };

        let tcp_task = {
            let shared = Arc::clone(&shared);
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = tcp.accept().await else {
                        continue;
                    };
                    let shared = Arc::clone(&shared);
                    tokio::spawn(serve_tcp(stream, shared));
                }
            })
        };

        FakeDns {
            addr,
            shared,
            tasks: vec![udp_task, tcp_task],
        }
    }

    /// Every question received so far, oldest first.
    pub fn queries(&self) -> Vec<Query> {
        self.shared.log.lock().unwrap().clone()
    }

    pub fn query_count(&self) -> usize {
        self.shared.log.lock().unwrap().len()
    }

    /// Questions received for one name (lowercase, no trailing dot).
    pub fn queries_for(&self, name: &str) -> Vec<Query> {
        self.queries()
            .into_iter()
            .filter(|q| q.name == name)
            .collect()
    }
}

impl Shared {
    fn record(&self, parsed: &ParsedQuery, transport: Transport) -> Query {
        let query = Query {
            name: parsed.name.clone(),
            qtype: parsed.qtype,
            transport,
            at: Instant::now(),
        };
        self.log.lock().unwrap().push(query.clone());
        query
    }
}

async fn serve_tcp(mut stream: TcpStream, shared: Arc<Shared>) {
    loop {
        let mut len_buf = [0u8; 2];
        if stream.read_exact(&mut len_buf).await.is_err() {
            return;
        }
        let len = u16::from_be_bytes(len_buf) as usize;
        let mut packet = vec![0u8; len];
        if stream.read_exact(&mut packet).await.is_err() {
            return;
        }
        let Some(parsed) = ParsedQuery::parse(&packet) else {
            return;
        };
        let query = shared.record(&parsed, Transport::Tcp);
        let reply = (shared.handler)(&query);
        for message in respond(&parsed, &reply, Transport::Tcp, shared.ttl).await {
            let mut framed = (message.len() as u16).to_be_bytes().to_vec();
            framed.extend_from_slice(&message);
            if stream.write_all(&framed).await.is_err() {
                return;
            }
        }
    }
}

/// The parts of a query the server needs to answer it.
struct ParsedQuery {
    id: u16,
    recursion_desired: bool,
    /// The raw question section (QNAME + QTYPE + QCLASS), echoed verbatim so
    /// the client's case and encoding match exactly.
    question: Vec<u8>,
    name: String,
    qtype: u16,
}

impl ParsedQuery {
    fn parse(packet: &[u8]) -> Option<Self> {
        if packet.len() < 12 {
            return None;
        }
        let id = u16::from_be_bytes([packet[0], packet[1]]);
        let flags = u16::from_be_bytes([packet[2], packet[3]]);
        let qdcount = u16::from_be_bytes([packet[4], packet[5]]);
        if qdcount != 1 {
            return None;
        }
        let mut pos = 12;
        let mut labels = Vec::new();
        loop {
            let len = *packet.get(pos)? as usize;
            pos += 1;
            if len == 0 {
                break;
            }
            if len & 0xC0 != 0 {
                return None; // queries never use compression
            }
            let label = packet.get(pos..pos + len)?;
            labels.push(String::from_utf8_lossy(label).to_ascii_lowercase());
            pos += len;
        }
        let qtype = u16::from_be_bytes([*packet.get(pos)?, *packet.get(pos + 1)?]);
        let end = pos + 4;
        let question = packet.get(12..end)?.to_vec();
        Some(ParsedQuery {
            id,
            recursion_desired: flags & 0x0100 != 0,
            question,
            name: labels.join("."),
            qtype,
        })
    }
}

/// The datagrams (UDP) or messages (TCP) to send for `reply`, after any delay.
async fn respond(
    query: &ParsedQuery,
    reply: &Reply,
    transport: Transport,
    ttl: u32,
) -> Vec<Vec<u8>> {
    match reply {
        Reply::Silent => Vec::new(),
        Reply::Garbage => vec![b"\xde\xad".to_vec()],
        Reply::Delayed(delay, inner) => {
            tokio::time::sleep(*delay).await;
            Box::pin(respond(query, inner, transport, ttl)).await
        }
        Reply::WrongIdThen { decoy, then } => {
            let wrong = build(
                query.id.wrapping_add(1),
                query,
                0,
                false,
                &records_for(query.qtype, std::slice::from_ref(decoy)),
                false,
                ttl,
            );
            let mut out = vec![wrong];
            tokio::time::sleep(Duration::from_millis(50)).await;
            out.extend(Box::pin(respond(query, then, transport, ttl)).await);
            out
        }
        Reply::TruncatedThenTcp(addrs) => match transport {
            Transport::Udp => vec![build(query.id, query, 0, true, &[], false, ttl)],
            Transport::Tcp => vec![build(
                query.id,
                query,
                0,
                false,
                &records_for(query.qtype, addrs),
                false,
                ttl,
            )],
        },
        Reply::Addresses(addrs) => {
            let records = records_for(query.qtype, addrs);
            let negative = records.is_empty();
            vec![build(query.id, query, 0, false, &records, negative, ttl)]
        }
        Reply::NxDomain => vec![build(
            query.id,
            query,
            RCODE_NXDOMAIN,
            false,
            &[],
            true,
            ttl,
        )],
        Reply::NoData => vec![build(query.id, query, 0, false, &[], true, ttl)],
        Reply::Rcode(code) => vec![build(query.id, query, *code, false, &[], false, ttl)],
    }
}

/// RDATA for every address of the family the question asked for.
fn records_for(qtype: u16, addrs: &[IpAddr]) -> Vec<Vec<u8>> {
    addrs
        .iter()
        .filter_map(|addr| match (qtype, addr) {
            (TYPE_A, IpAddr::V4(v4)) => Some(v4.octets().to_vec()),
            (TYPE_AAAA, IpAddr::V6(v6)) => Some(v6.octets().to_vec()),
            _ => None,
        })
        .collect()
}

fn build(
    id: u16,
    query: &ParsedQuery,
    rcode: u8,
    truncated: bool,
    answers: &[Vec<u8>],
    with_soa: bool,
    ttl: u32,
) -> Vec<u8> {
    let mut flags: u16 = 0x8000 | 0x0080 | u16::from(rcode & 0x0F); // QR, RA
    if query.recursion_desired {
        flags |= 0x0100;
    }
    if truncated {
        flags |= 0x0200;
    }
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    out.extend_from_slice(&(answers.len() as u16).to_be_bytes()); // ANCOUNT
    out.extend_from_slice(&u16::from(with_soa).to_be_bytes()); // NSCOUNT
    out.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT
    out.extend_from_slice(&query.question);
    for rdata in answers {
        out.extend_from_slice(&[0xC0, 0x0C]); // pointer to the question's name
        out.extend_from_slice(&query.qtype.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes()); // class IN
        out.extend_from_slice(&ttl.to_be_bytes());
        out.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        out.extend_from_slice(rdata);
    }
    if with_soa {
        // An SOA at the root carries the negative-caching TTL (RFC 2308).
        out.push(0); // owner: root
        out.extend_from_slice(&6u16.to_be_bytes()); // type SOA
        out.extend_from_slice(&1u16.to_be_bytes()); // class IN
        out.extend_from_slice(&ttl.to_be_bytes());
        out.extend_from_slice(&22u16.to_be_bytes()); // RDLENGTH
        out.push(0); // MNAME: root
        out.push(0); // RNAME: root
        for value in [1u32, 3600, 600, 86400, ttl] {
            out.extend_from_slice(&value.to_be_bytes()); // serial..minimum
        }
    }
    out
}

/// A minimal HTTP/1.1 server on `127.0.0.1` that answers every request with
/// `200 OK` and `body`, and counts requests. Used as the target the resolved
/// name points at, so a test can tell "resolved and connected" apart from
/// "failed to resolve".
pub struct FakeHttp {
    pub port: u16,
    requests: Arc<Mutex<usize>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for FakeHttp {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl FakeHttp {
    pub async fn start(body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind http");
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(0usize));
        let task = {
            let requests = Arc::clone(&requests);
            tokio::spawn(async move {
                loop {
                    let Ok((mut stream, _)) = listener.accept().await else {
                        continue;
                    };
                    let requests = Arc::clone(&requests);
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 8192];
                        let mut seen = Vec::new();
                        while !seen.windows(4).any(|w| w == b"\r\n\r\n") {
                            match stream.read(&mut buf).await {
                                Ok(0) | Err(_) => return,
                                Ok(n) => seen.extend_from_slice(&buf[..n]),
                            }
                        }
                        *requests.lock().unwrap() += 1;
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write_all(response.as_bytes()).await;
                        let _ = stream.shutdown().await;
                    });
                }
            })
        };
        FakeHttp {
            port,
            requests,
            task,
        }
    }

    pub fn request_count(&self) -> usize {
        *self.requests.lock().unwrap()
    }
}
