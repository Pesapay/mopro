//! SpeakUp zk-vm (mpz `vm-zk`) for Mopro: an interactive, designated-verifier VOLE-ZK
//! prover and verifier for WebAssembly guests, talking over TCP.
//!
//! Guests are the SpeakUp demo programs (`test-vectors/speakup`). The OT stack is the
//! production one: Ferret over SoftSpoken over Chou-Orlandi.

use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    task::{Context as TaskCx, Poll},
    time::{Duration, Instant},
};

use futures::{AsyncRead, AsyncWrite};
use mpz_common::Context;
use mpz_core::Block;
use mpz_ot::{chou_orlandi, ferret, softspoken};
use mpz_vm_core::{value::Value, Param, ValType, Vm, Write};
use mpz_vm_ir::{ExportKind, Module};
use mpz_vm_zk::{Config, Prover, Verifier};
use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::MoproError;

type ProverSvole = ferret::Receiver<softspoken::Receiver<chou_orlandi::Sender>>;
type VerifierSvole = ferret::Sender<softspoken::Sender<chou_orlandi::Receiver>>;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const ACCEPT_TIMEOUT: Duration = Duration::from_secs(120);

fn err(e: impl std::fmt::Display) -> MoproError {
    MoproError::SpeakupError(e.to_string())
}

/// One call argument: `kind` is `public|private|blind`, `ty` is `i32|i64`.
/// `value` is ignored for `blind` (the verifier's view of a private argument).
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Clone, Debug)]
pub struct SpeakupArg {
    pub kind: String,
    pub ty: String,
    pub value: i64,
}

#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Clone, Debug, Default)]
pub struct SpeakupWireStats {
    pub sent_bytes: u64,
    pub received_bytes: u64,
    pub flushes: u64,
    pub setup_ms: u64,
}

/// Result of a sha256 proving session, as seen by the prover.
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Clone, Debug)]
pub struct SpeakupBenchResult {
    pub digest_hex: String,
    pub total_ms: u64,
    pub prover_sent_bytes: u64,
    pub prover_received_bytes: u64,
    pub verifier_accepted: bool,
}

fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("speakup-io")
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

/// Sizes rayon's global pool. Call before the first session; later calls are no-ops.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn speakup_init_threads(threads: u32) {
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1) as usize)
        .build_global();
}

/// The SpeakUp demo guests: `sha256` (export `hash`) and `sudoku` (export `check`).
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn speakup_builtin_guest(name: String) -> Result<Vec<u8>, MoproError> {
    match name.as_str() {
        "sha256" => Ok(include_bytes!("../test-vectors/speakup/speakup_sha256.wasm").to_vec()),
        "sudoku" => Ok(include_bytes!("../test-vectors/speakup/speakup_sudoku.wasm").to_vec()),
        other => Err(err(format!("no builtin guest: {other}"))),
    }
}

#[derive(Clone, Default)]
struct Wire {
    tx: Arc<AtomicU64>,
    rx: Arc<AtomicU64>,
    flushes: Arc<AtomicU64>,
}

struct Counted<S> {
    inner: S,
    wire: Wire,
}

impl<S: AsyncRead + Unpin> AsyncRead for Counted<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut TaskCx<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let r = Pin::new(&mut this.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(n)) = &r {
            this.wire.rx.fetch_add(*n as u64, Ordering::Relaxed);
        }
        r
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Counted<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut TaskCx<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let r = Pin::new(&mut this.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = &r {
            this.wire.tx.fetch_add(*n as u64, Ordering::Relaxed);
        }
        r
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskCx<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let r = Pin::new(&mut this.inner).poll_flush(cx);
        if let Poll::Ready(Ok(())) = &r {
            this.wire.flushes.fetch_add(1, Ordering::Relaxed);
        }
        r
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut TaskCx<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_close(cx)
    }
}

enum Engine {
    Prover(Prover<ProverSvole>),
    Verifier(Verifier<VerifierSvole>),
}

struct Inner {
    ctx: Context,
    engine: Engine,
}

/// A prover or verifier bound to one peer connection. Calls must not overlap.
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct SpeakupParty {
    module: Module,
    inner: Mutex<Option<Inner>>,
    wire: Wire,
    setup_ms: u64,
}

fn prover_svole() -> ProverSvole {
    let mut rng = StdRng::from_os_rng();
    ferret::Receiver::new(
        ferret::FerretConfig::default(),
        rng.random(),
        softspoken::Receiver::new(
            softspoken::ReceiverConfig::default(),
            chou_orlandi::Sender::new(),
        ),
    )
}

fn verifier_svole() -> VerifierSvole {
    let mut rng = StdRng::from_os_rng();
    let mut delta: Block = rng.random();
    delta.set_lsb(true);
    ferret::Sender::new(
        ferret::FerretConfig::default(),
        rng.random(),
        softspoken::Sender::new(
            softspoken::SenderConfig::default(),
            delta,
            chou_orlandi::Receiver::new(),
        ),
    )
}

fn open(
    stream: TcpStream,
    wasm: &[u8],
    prover: bool,
    chunk_cap: Option<u64>,
    t0: Instant,
) -> Result<Arc<SpeakupParty>, MoproError> {
    stream.set_nodelay(true).map_err(err)?;
    let module = Module::parse(wasm).map_err(err)?;
    let wire = Wire::default();
    let ctx = Context::new_single_threaded(Counted {
        inner: stream.compat(),
        wire: wire.clone(),
    });
    let cfg = Config::builder()
        .chunk_cap(chunk_cap.map(|c| c as usize))
        .build();
    let engine = if prover {
        Engine::Prover(Prover::new_with_config(module.clone(), prover_svole(), cfg).map_err(err)?)
    } else {
        Engine::Verifier(
            Verifier::new_with_config(module.clone(), verifier_svole(), cfg).map_err(err)?,
        )
    };
    Ok(Arc::new(SpeakupParty {
        module,
        inner: Mutex::new(Some(Inner { ctx, engine })),
        wire,
        setup_ms: t0.elapsed().as_millis() as u64,
    }))
}

async fn spawn<T: Send + 'static>(
    fut: impl std::future::Future<Output = Result<T, MoproError>> + Send + 'static,
) -> Result<T, MoproError> {
    rt().spawn(fut)
        .await
        .map_err(|e| err(format!("task failed: {e}")))?
}

async fn listen(addr: SocketAddr) -> Result<TcpListener, MoproError> {
    let socket = if addr.is_ipv4() {
        TcpSocket::new_v4()
    } else {
        TcpSocket::new_v6()
    }
    .map_err(err)?;
    // The verifier closes first and leaves its port in TIME_WAIT; allow an immediate rerun.
    socket.set_reuseaddr(true).map_err(err)?;
    socket.bind(addr).map_err(err)?;
    socket.listen(1).map_err(err)
}

async fn accept(
    listener: TcpListener,
    wasm: Vec<u8>,
    chunk_cap: Option<u64>,
) -> Result<Arc<SpeakupParty>, MoproError> {
    let (stream, _) = tokio::time::timeout(ACCEPT_TIMEOUT, listener.accept())
        .await
        .map_err(|_| err("no prover connected"))?
        .map_err(err)?;
    open(stream, &wasm, false, chunk_cap, Instant::now())
}

/// Connects to a verifier at `addr` (host:port), retrying while it comes up.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub async fn speakup_prover_connect(
    addr: String,
    wasm: Vec<u8>,
    chunk_cap: Option<u64>,
) -> Result<Arc<SpeakupParty>, MoproError> {
    spawn(async move {
        let t0 = Instant::now();
        let stream = loop {
            match TcpStream::connect(&addr).await {
                Ok(s) => break s,
                Err(e) if t0.elapsed() > CONNECT_TIMEOUT => return Err(err(e)),
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        };
        open(stream, &wasm, true, chunk_cap, t0)
    })
    .await
}

/// Listens on `addr` and becomes the verifier for the first prover to connect.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub async fn speakup_verifier_accept(
    addr: String,
    wasm: Vec<u8>,
    chunk_cap: Option<u64>,
) -> Result<Arc<SpeakupParty>, MoproError> {
    spawn(async move {
        let sock_addr = tokio::net::lookup_host(&addr)
            .await
            .map_err(err)?
            .next()
            .ok_or_else(|| err(format!("bad address {addr}")))?;
        accept(listen(sock_addr).await?, wasm, chunk_cap).await
    })
    .await
}

impl SpeakupParty {
    fn with<T>(
        &self,
        f: impl FnOnce(&mut Inner) -> Result<T, MoproError>,
    ) -> Result<T, MoproError> {
        let mut slot = self.inner.lock().unwrap();
        f(slot
            .as_mut()
            .ok_or_else(|| err("party is busy (a call is in flight) or closed"))?)
    }

    fn write(&self, offset: u32, w: Write<'_>) -> Result<(), MoproError> {
        self.with(|i| {
            match &mut i.engine {
                Engine::Prover(p) => p.write(offset, w),
                Engine::Verifier(v) => v.write(offset, w),
            }
            .map_err(err)
        })
    }
}

// Argument names avoid `ptr`: it collides with the object handle in UniFFI's C header.
#[cfg_attr(feature = "uniffi", uniffi::export)]
impl SpeakupParty {
    pub fn is_prover(&self) -> Result<bool, MoproError> {
        self.with(|i| Ok(matches!(i.engine, Engine::Prover(_))))
    }

    /// Runs a non-interactive export (e.g. `cabi_realloc`) on this side only.
    pub fn call_local(
        &self,
        name: String,
        args: Vec<SpeakupArg>,
    ) -> Result<Option<i64>, MoproError> {
        let idx = func(&self.module, &name)?;
        let params = params(&args)?;
        self.with(|i| {
            match &mut i.engine {
                Engine::Prover(p) => p.call_local(idx, params),
                Engine::Verifier(v) => v.call_local(idx, params),
            }
            .map_err(err)
            .and_then(scalar)
        })
    }

    /// Prover only: stage private bytes. The verifier stages the same length with `write_blind`.
    pub fn write_private(&self, offset: u32, bytes: Vec<u8>) -> Result<(), MoproError> {
        self.write(offset, Write::Private(&bytes))
    }

    pub fn write_public(&self, offset: u32, bytes: Vec<u8>) -> Result<(), MoproError> {
        self.write(offset, Write::Public(&bytes))
    }

    pub fn write_blind(&self, offset: u32, len: u32) -> Result<(), MoproError> {
        self.write(offset, Write::Blind(len as usize))
    }

    /// Reads revealed or public bytes.
    pub fn read(&self, offset: u32, len: u32) -> Result<Vec<u8>, MoproError> {
        self.with(|i| {
            match &i.engine {
                Engine::Prover(p) => p.read(offset, len as usize),
                Engine::Verifier(v) => v.read(offset, len as usize),
            }
            .map(<[u8]>::to_vec)
            .map_err(err)
        })
    }

    pub fn wire_stats(&self) -> SpeakupWireStats {
        SpeakupWireStats {
            sent_bytes: self.wire.tx.load(Ordering::Relaxed),
            received_bytes: self.wire.rx.load(Ordering::Relaxed),
            flushes: self.wire.flushes.load(Ordering::Relaxed),
            setup_ms: self.setup_ms,
        }
    }

    /// One interactive protocol round; the peer must issue the matching call.
    /// A failed round closes the party, so the peer sees EOF instead of hanging.
    pub async fn call(
        &self,
        name: String,
        args: Vec<SpeakupArg>,
    ) -> Result<Option<i64>, MoproError> {
        let idx = func(&self.module, &name)?;
        let params = params(&args)?;
        let mut inner = self
            .inner
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| err("party is busy (a call is in flight) or closed"))?;
        let (inner, res) = rt()
            .spawn(async move {
                let r = match &mut inner.engine {
                    Engine::Prover(p) => p.call(&mut inner.ctx, idx, params).await,
                    Engine::Verifier(v) => v.call(&mut inner.ctx, idx, params).await,
                }
                .map_err(err);
                (inner, r)
            })
            .await
            .map_err(|e| err(format!("call task failed, party closed: {e}")))?;
        if res.is_ok() {
            *self.inner.lock().unwrap() = Some(inner);
        }
        res.and_then(scalar)
    }
}

fn func(module: &Module, name: &str) -> Result<u32, MoproError> {
    module
        .exports()
        .iter()
        .find_map(|e| match e.kind {
            ExportKind::Func(idx) if e.name == name => Some(idx),
            _ => None,
        })
        .ok_or_else(|| err(format!("export not found: {name}")))
}

fn params(args: &[SpeakupArg]) -> Result<Vec<Param>, MoproError> {
    args.iter()
        .map(|a| {
            let (ty, value) = match a.ty.as_str() {
                "i32" => (ValType::I32, Value::I32(a.value as i32)),
                "i64" => (ValType::I64, Value::I64(a.value)),
                other => return Err(err(format!("only i32/i64 params, got {other}"))),
            };
            Ok(match a.kind.as_str() {
                "public" => Param::Public(value),
                "private" => Param::Private(value),
                "blind" => Param::Blind(ty),
                other => return Err(err(format!("unknown param kind: {other}"))),
            })
        })
        .collect()
}

fn scalar(v: Option<Value>) -> Result<Option<i64>, MoproError> {
    match v {
        None => Ok(None),
        Some(Value::I32(x)) => Ok(Some(x as i64)),
        Some(Value::I64(x)) => Ok(Some(x)),
        Some(other) => Err(err(format!("float result not supported: {other:?}"))),
    }
}

fn public(v: i64) -> SpeakupArg {
    SpeakupArg {
        kind: "public".into(),
        ty: "i32".into(),
        value: v,
    }
}

/// The demo's sha256 script, identical on both sides: `msg` is `None` on the verifier.
async fn sha256_session(
    party: &SpeakupParty,
    len: u32,
    msg: Option<Vec<u8>>,
) -> Result<Vec<u8>, MoproError> {
    let alloc = vec![public(0), public(0), public(1), public(len as i64 + 32)];
    let ptr = party
        .call_local("cabi_realloc".into(), alloc)?
        .ok_or_else(|| err("cabi_realloc returned nothing"))? as u32;
    match msg {
        Some(m) => party.write_private(ptr, m)?,
        None => party.write_blind(ptr, len)?,
    }
    let digest_ptr = party
        .call("hash".into(), vec![public(ptr as i64), public(len as i64)])
        .await?
        .ok_or_else(|| err("hash returned nothing"))? as u32;
    party.read(digest_ptr, 32)
}

fn filler(len: u32) -> Vec<u8> {
    (0..len)
        .map(|i| (i.wrapping_mul(7).wrapping_add(3)) as u8)
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Proves SHA-256 of `len` private bytes with prover and verifier both on this device
/// (loopback TCP). Blocking; call it off the UI thread.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn speakup_prove_sha256_loopback(
    len: u32,
    chunk_cap: Option<u64>,
) -> Result<SpeakupBenchResult, MoproError> {
    let wasm = speakup_builtin_guest("sha256".into())?;
    futures::executor::block_on(async move {
        let t0 = Instant::now();
        let listener = spawn(listen("127.0.0.1:0".parse().unwrap())).await?;
        let addr = listener.local_addr().map_err(err)?.to_string();
        let (v, p) = futures::future::join(
            spawn(accept(listener, wasm.clone(), chunk_cap)),
            speakup_prover_connect(addr, wasm, chunk_cap),
        )
        .await;
        let (v, p) = (v?, p?);
        let (vd, pd) = futures::future::join(
            sha256_session(&v, len, None),
            sha256_session(&p, len, Some(filler(len))),
        )
        .await;
        let (vd, pd) = (vd?, pd?);
        let w = p.wire_stats();
        Ok(SpeakupBenchResult {
            digest_hex: hex(&pd),
            total_ms: t0.elapsed().as_millis() as u64,
            prover_sent_bytes: w.sent_bytes,
            prover_received_bytes: w.received_bytes,
            verifier_accepted: vd == pd,
        })
    })
}

/// Proves SHA-256 of `len` private bytes to a remote verifier at `addr` running the
/// same guest and length. `verifier_accepted` is not observable from the prover side.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn speakup_prove_sha256_remote(
    addr: String,
    len: u32,
    chunk_cap: Option<u64>,
) -> Result<SpeakupBenchResult, MoproError> {
    let wasm = speakup_builtin_guest("sha256".into())?;
    futures::executor::block_on(async move {
        let t0 = Instant::now();
        let p = speakup_prover_connect(addr, wasm, chunk_cap).await?;
        let digest = sha256_session(&p, len, Some(filler(len))).await?;
        let w = p.wire_stats();
        Ok(SpeakupBenchResult {
            digest_hex: hex(&digest),
            total_ms: t0.elapsed().as_millis() as u64,
            prover_sent_bytes: w.sent_bytes,
            prover_received_bytes: w.received_bytes,
            verifier_accepted: false,
        })
    })
}

/// Serves the sha256 guest as verifier to one prover at `addr`; returns the revealed digest.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn speakup_verify_sha256_once(
    addr: String,
    len: u32,
    chunk_cap: Option<u64>,
) -> Result<String, MoproError> {
    let wasm = speakup_builtin_guest("sha256".into())?;
    futures::executor::block_on(async move {
        let v = speakup_verifier_accept(addr, wasm, chunk_cap).await?;
        Ok(hex(&sha256_session(&v, len, None).await?))
    })
}
