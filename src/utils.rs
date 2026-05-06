use core::sync;
use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
    marker::PhantomData,
    net::IpAddr,
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::{Arc, Weak, atomic::AtomicBool},
    task::Waker,
    time::Duration,
};

use crate::try_catch;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{Mutex, MutexGuard},
};

pub struct Stream<T: Serialize + DeserializeOwned> {
    data: PhantomData<T>,
    stream: Mutex<TcpStream>,
    addr: IpAddr,
    has_failed_at_some_point: AtomicBool,
}
impl<T: DeserializeOwned + Serialize> Stream<T> {
    pub fn new(st: TcpStream) -> Self {
        let addr = st.local_addr().unwrap().ip();
        Self {
            data: Default::default(),
            stream: Mutex::new(st),
            addr,
            has_failed_at_some_point: AtomicBool::new(false),
        }
    }

    pub async fn send(&self, value: &T) -> Result<usize, tokio::io::Error> {
        let v = rmp_serde::to_vec(value).unwrap();
        let count = v.len();
        let mut guard = self.stream.lock().await;
        if let Err(e) = guard.write_u64_le(count as u64).await {
            match e.kind() {
                std::io::ErrorKind::ConnectionReset => {}
                std::io::ErrorKind::WouldBlock => {}
                _ => {
                    self.has_failed_at_some_point
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
            return Err(e);
        };
        if let Err(e) = guard.write_all(&v).await {
            match e.kind() {
                std::io::ErrorKind::ConnectionReset => {}
                std::io::ErrorKind::WouldBlock => {}
                _ => {
                    self.has_failed_at_some_point
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
            return Err(e);
        };
        Ok(count)
    }

    pub fn send_blocking(&self, value: &T) -> Result<usize, tokio::io::Error> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(self.send(value))
    }

    pub async fn receive(&self) -> Result<T, tokio::io::Error> {
        let (mut guard, count) = loop {
            let mut guard = self.stream.lock().await;
            let count = match guard.read_u64_le().await {
                Ok(x) => x,
                Err(y) => match y.kind() {
                    std::io::ErrorKind::ConnectionReset => {
                        continue;
                    }
                    _ => {
                        self.has_failed_at_some_point
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                        return Err(y);
                    }
                },
            } as usize;
            break (guard, count);
        };
        let mut buffer = vec![0u8; count];
        match guard.read_exact(&mut buffer).await {
            Ok(x) => x,
            Err(y) => {
                self.has_failed_at_some_point
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                return Err(y);
            }
        };
        let value = rmp_serde::from_slice(&buffer);
        match value {
            Ok(t) => Ok(t),
            Err(_) => Err(tokio::io::Error::new(
                std::io::ErrorKind::InvalidData,
                String::from("failed to deserialize"),
            )),
        }
    }

    pub async fn try_receive(&self) -> Result<Option<T>, tokio::io::Error> {
        let mut guard = self.stream.lock().await;
        let mut count = [0u8; 8];
        let Ok(g) = tokio::time::timeout(Duration::from_micros(1), guard.peek(&mut count)).await
        else {
            return Ok(None);
        };
        match g {
            Ok(x) => {
                if x == 8 {
                    let count = match guard.read_u64_le().await {
                        Ok(x) => x as usize,
                        Err(e) => match e.kind() {
                            tokio::io::ErrorKind::ConnectionReset => {
                                return Ok(None);
                            }
                            tokio::io::ErrorKind::WouldBlock => {
                                return Ok(None);
                            }
                            _ => {
                                self.has_failed_at_some_point
                                    .store(true, std::sync::atomic::Ordering::SeqCst);
                                return Err(e);
                            }
                        },
                    };
                    let mut v = vec![0u8; count as usize];
                    match guard.read_exact(&mut v).await {
                        Ok(x) => x,
                        Err(e) => match e.kind() {
                            tokio::io::ErrorKind::ConnectionReset => {
                                return Ok(None);
                            }
                            tokio::io::ErrorKind::WouldBlock => {
                                return Ok(None);
                            }
                            _ => {
                                self.has_failed_at_some_point
                                    .store(true, std::sync::atomic::Ordering::SeqCst);
                                return Err(e);
                            }
                        },
                    };
                    let Ok(out) = rmp_serde::from_slice(&v) else {
                        return Err(tokio::io::Error::new(
                            tokio::io::ErrorKind::InvalidData,
                            "invalid rmp message".to_string(),
                        ));
                    };
                    Ok(Some(out))
                } else {
                    Ok(None)
                }
            }
            Err(x) => match x.kind() {
                std::io::ErrorKind::ConnectionReset => Ok(None),
                std::io::ErrorKind::WouldBlock => Ok(None),
                _ => Err(x),
            },
        }
    }

    pub fn receive_blocking(&self) -> Result<T, tokio::io::Error> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(self.receive())
    }

    pub fn try_receive_blocking(&self) -> Result<Option<T>, tokio::io::Error> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(self.try_receive())
    }

    pub fn has_errored_fatally(&self) -> bool {
        self.has_failed_at_some_point
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub struct BPipe<T> {
    source: Arc<std::sync::Mutex<VecDeque<T>>>,
    sync: Arc<std::sync::Mutex<VecDeque<T>>>,
    waker: Arc<std::sync::Mutex<Option<Waker>>>,
    thread_waker: Arc<std::sync::Mutex<Option<std::thread::Thread>>>,
}
impl<T> Drop for BPipe<T> {
    fn drop(&mut self) {
        let mut waker = match self.waker.lock() {
            Ok(x) => x,
            Err(x) => x.into_inner(),
        };
        if let Some(wake) = waker.take() {
            wake.wake();
        }
        let mut thread = match self.thread_waker.lock() {
            Ok(x) => x,
            Err(x) => x.into_inner(),
        };
        if let Some(thread) = thread.take() {
            thread.unpark();
        }
    }
}
impl<T> BPipe<T> {
    pub fn create() -> (Self, Self) {
        let s1 = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let s2 = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let t1 = s2.clone();
        let t2 = s1.clone();
        let waker = Arc::new(std::sync::Mutex::new(None));
        let thread_waker = Arc::new(std::sync::Mutex::new(None));
        (
            Self {
                source: s1,
                sync: s2,
                waker: waker.clone(),
                thread_waker: thread_waker.clone(),
            },
            Self {
                source: t1,
                sync: t2,
                waker,
                thread_waker,
            },
        )
    }

    pub fn send(&self, value: T) {
        let mut guard = match self.sync.lock() {
            Ok(x) => x,
            Err(x) => x.into_inner(),
        };
        guard.push_back(value);
        let mut waker = match self.waker.lock() {
            Ok(x) => x,
            Err(x) => x.into_inner(),
        };
        if let Some(wake) = waker.take() {
            wake.wake();
        }
        let mut thread = match self.thread_waker.lock() {
            Ok(x) => x,
            Err(x) => x.into_inner(),
        };
        if let Some(thread) = thread.take() {
            thread.unpark();
        }
    }

    pub fn try_receive(&self) -> Option<T> {
        let mut guard = match self.source.lock() {
            Ok(x) => x,
            Err(x) => x.into_inner(),
        };
        guard.pop_front()
    }

    pub fn has_other(&self) -> bool {
        Arc::strong_count(&self.sync) > 1
    }

    pub fn receive<'a>(&'a self) -> BPipeRecieveFuture<'a, T> {
        BPipeRecieveFuture { ptr: self }
    }

    pub fn receive_blocking(&self) -> Result<T, std::io::Error> {
        while self.has_other() {
            if let Some(x) = self.try_receive() {
                return Ok(x);
            }
            {
                let guard = match self.waker.lock() {
                    Ok(x) => x,
                    Err(x) => x.into_inner(),
                };
                if let Some(_guard) = guard.as_ref() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Deadlock,
                        String::from("would Deadlock"),
                    ));
                }
                let mut guard = match self.thread_waker.lock() {
                    Ok(x) => x,
                    Err(x) => x.into_inner(),
                };
                if let Some(_guard) = guard.as_ref() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Deadlock,
                        String::from("would Deadlock"),
                    ));
                }
                *guard = Some(std::thread::current());
            }
            std::thread::park();
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            String::from("broken pipe"),
        ))
    }
}

pub struct BPipeRecieveFuture<'a, T> {
    ptr: &'a BPipe<T>,
}
impl<'a, T> Future for BPipeRecieveFuture<'a, T> {
    type Output = Result<T, std::io::Error>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let ptr = self.ptr;
        if !ptr.has_other() {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                String::from("pipe broken"),
            )))
        } else {
            let mut guard = match ptr.source.lock() {
                Ok(x) => x,
                Err(x) => x.into_inner(),
            };
            if let Some(x) = guard.pop_front() {
                std::task::Poll::Ready(Ok(x))
            } else {
                let wk = cx.waker();
                let mut waker = match ptr.waker.lock() {
                    Ok(x) => x,
                    Err(e) => e.into_inner(),
                };
                if waker.is_some() {
                    return std::task::Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        String::from("pipe broken"),
                    )));
                }
                *waker = Some(wk.clone());
                let thread_waker = match ptr.thread_waker.lock() {
                    Ok(x) => x,
                    Err(x) => x.into_inner(),
                };
                if thread_waker.is_some() {
                    return std::task::Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        String::from("pipe broken"),
                    )));
                }
                std::task::Poll::Pending
            }
        }
    }
}

impl<T> Iterator for BPipe<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.try_receive()
    }
}

impl<T: serde::Serialize + serde::de::DeserializeOwned> Iterator for Stream<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.try_receive_blocking().unwrap_or_default()
    }
}
enum BStreamInner<T: Serialize + DeserializeOwned> {
    Network(Stream<T>),
    Local(BPipe<T>),
}
pub struct BStream<T: Serialize + DeserializeOwned + Clone> {
    inner: BStreamInner<T>,
}
impl<T: Serialize + DeserializeOwned + Clone> BStream<T> {
    pub fn from_stream(s: Stream<T>) -> Self {
        Self {
            inner: BStreamInner::Network(s),
        }
    }
    pub fn from_pipe(s: BPipe<T>) -> Self {
        Self {
            inner: BStreamInner::Local(s),
        }
    }
    pub async fn send(&self, value: &T) -> Result<(), tokio::io::Error> {
        match &self.inner {
            BStreamInner::Local(v) => {
                v.send(value.clone());
            }
            BStreamInner::Network(v) => {
                v.send(value).await?;
            }
        }
        Ok(())
    }

    pub fn send_blocking(&self, value: &T) -> Result<(), tokio::io::Error> {
        match &self.inner {
            BStreamInner::Local(v) => {
                v.send(value.clone());
            }
            BStreamInner::Network(v) => {
                v.send_blocking(value)?;
            }
        }
        Ok(())
    }

    pub async fn receive(&self) -> Result<T, tokio::io::Error> {
        match &self.inner {
            BStreamInner::Local(v) => v.receive().await,
            BStreamInner::Network(v) => v.receive().await,
        }
    }

    pub async fn try_receive(&self) -> Result<Option<T>, tokio::io::Error> {
        match &self.inner {
            BStreamInner::Local(v) => Ok(v.try_receive()),
            BStreamInner::Network(v) => v.try_receive().await,
        }
    }

    pub fn receive_blocking(&self) -> Result<T, tokio::io::Error> {
        match &self.inner {
            BStreamInner::Local(v) => v.receive_blocking(),
            BStreamInner::Network(v) => v.receive_blocking(),
        }
    }

    pub fn try_receive_blocking(&self) -> Result<Option<T>, tokio::io::Error> {
        match &self.inner {
            BStreamInner::Local(v) => Ok(v.try_receive()),
            BStreamInner::Network(v) => v.try_receive_blocking(),
        }
    }

    pub fn has_errored_fatally(&self) -> bool {
        match &self.inner {
            BStreamInner::Local(v) => !v.has_other(),
            BStreamInner::Network(n) => n.has_errored_fatally(),
        }
    }

    pub fn get_ip_address(&self) -> Option<IpAddr> {
        match &self.inner {
            BStreamInner::Network(stream) => Some(stream.addr),
            BStreamInner::Local(_) => None,
        }
    }
}
#[tokio::test]
pub async fn pipe_test() {
    use tokio::task::yield_now;
    async fn test(pipe: BPipe<i32>) {
        let mut primes: Vec<i32> = Vec::new();
        let mut current = 2;
        loop {
            if let Some(_) = pipe.try_receive() {
                break;
            }
            let mut is_prime = true;
            for j in &primes {
                if current % *j == 0 {
                    is_prime = false;
                    break;
                }
            }
            if is_prime {
                pipe.send(current);
                primes.push(current);
            }
            current += 1;
            yield_now().await;
        }
    }
    let mut pipes = Vec::new();
    let mut futures = Vec::new();
    for _ in 0..4 {
        let (t1, t2) = BPipe::<i32>::create();
        pipes.push(t1);
        futures.push(tokio::spawn(test(t2)));
    }
    loop {
        let mut hit = false;
        for (idx, i) in pipes.iter().enumerate() {
            let x0 = i.receive().await.unwrap();
            {
                use std::io::Write;
                writeln!(std::io::stderr(), "async idx:{} value:{}", idx, x0).unwrap();
            }
            if x0 > 100 {
                hit = true;
            }
        }
        if hit {
            for j in &pipes {
                j.send(-1);
            }
            for i in futures {
                i.await.unwrap();
            }
            break;
        }
    }
}

#[test]
pub fn pipe_test_sync() {
    fn test(pipe: BPipe<i32>) {
        let mut primes: Vec<i32> = Vec::new();
        let mut current = 2;
        loop {
            if let Some(_) = pipe.try_receive() {
                break;
            }
            let mut is_prime = true;
            for j in &primes {
                if current % *j == 0 {
                    is_prime = false;
                    break;
                }
            }
            if is_prime {
                pipe.send(current);
                primes.push(current);
            }
            current += 1;
            std::thread::yield_now();
        }
    }
    let mut pipes = Vec::new();
    let mut futures = Vec::new();
    for _ in 0..4 {
        let (t1, t2) = BPipe::<i32>::create();
        pipes.push(t1);
        futures.push(std::thread::spawn(|| {
            test(t2);
        }));
    }
    loop {
        let mut hit = false;
        for (idx, i) in pipes.iter().enumerate() {
            let x0 = i.receive_blocking().unwrap();
            {
                use std::io::Write;
                writeln!(std::io::stderr(), "sync idx:{} value:{}", idx, x0).unwrap();
            }
            if x0 > 100 {
                hit = true;
            }
        }
        if hit {
            for j in &pipes {
                j.send(-1);
            }
            for i in futures {
                i.join().unwrap();
            }
            break;
        }
    }
}

#[test]
#[should_panic]
pub fn pipe_test_sync_fail() {
    fn test(pipe: BPipe<i32>) {
        let mut primes: Vec<i32> = Vec::new();
        let mut current = 2;
        loop {
            if let Some(_) = pipe.try_receive() {
                break;
            }
            let mut is_prime = true;
            for j in &primes {
                if current % *j == 0 {
                    is_prime = false;
                    break;
                }
            }
            if is_prime {
                pipe.send(current);
                primes.push(current);
            }
            current += 1;
            if current == 93 {
                pipe.receive_blocking().unwrap();
            }
            std::thread::yield_now();
        }
    }
    let mut pipes = Vec::new();
    let mut futures = Vec::new();
    for _ in 0..4 {
        let (t1, t2) = BPipe::<i32>::create();
        pipes.push(t1);
        futures.push(std::thread::spawn(|| {
            test(t2);
        }));
    }
    loop {
        let mut hit = false;
        for (idx, i) in pipes.iter().enumerate() {
            let x0 = i.receive_blocking().unwrap();
            {
                use std::io::Write;
                writeln!(std::io::stderr(), "sync fail idx:{} value:{}", idx, x0).unwrap();
            }
            if x0 > 100 {
                hit = true;
            }
        }
        if hit {
            for j in &pipes {
                j.send(-1);
            }
            for i in futures {
                i.join().unwrap();
            }
            break;
        }
    }
}

#[tokio::test]
#[should_panic]
pub async fn pipe_test_fail() {
    use tokio::task::yield_now;
    async fn test(pipe: BPipe<i32>) {
        let mut primes: Vec<i32> = Vec::new();
        let mut current = 2;
        loop {
            if let Some(_) = pipe.try_receive() {
                break;
            }
            let mut is_prime = true;
            for j in &primes {
                if current % *j == 0 {
                    is_prime = false;
                    break;
                }
            }
            if is_prime {
                pipe.send(current);
                primes.push(current);
            }
            current += 1;
            if current == 93 {
                pipe.receive().await.unwrap();
            }
            yield_now().await;
        }
    }
    let mut pipes = Vec::new();
    let mut futures = Vec::new();
    for _ in 0..4 {
        let (t1, t2) = BPipe::<i32>::create();
        pipes.push(t1);
        futures.push(tokio::spawn(test(t2)));
    }
    loop {
        let mut hit = false;
        for (idx, i) in pipes.iter().enumerate() {
            let x0 = i.receive().await.unwrap();
            {
                use std::io::Write;
                writeln!(std::io::stderr(), "async fail idx:{} value:{}", idx, x0).unwrap();
            }
            if x0 > 100 {
                hit = true;
            }
        }
        if hit {
            for j in &pipes {
                j.send(-1);
            }
            for i in futures {
                i.await.unwrap();
            }
            break;
        }
    }
}

static HWID: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
pub fn generate_id() -> ObjectId {
    let mut _hwid = match HWID.lock() {
        Ok(x) => x,
        Err(e) => e.into_inner(),
    };
    if _hwid.is_none() {
        *_hwid = Some(hardware_id::get_id().unwrap());
    }
    let hwid = _hwid.as_ref().unwrap();
    let value = std::time::SystemTime::now();
    //if this is not unique idk what is
    ObjectId {
        id: Some(
            format!(
                "{}-{:#?}-{:#?}-{:#?}",
                hwid,
                std::thread::current().id(),
                std::process::id(),
                value
            )
            .into(),
        ),
    }
}

#[test]
pub fn id_test() {
    let mut list = std::collections::HashSet::new();
    for _ in 0..10000 {
        let tmp = generate_id();
        assert!(!list.contains(&tmp));
        //writeln!(stderr(), "{:#?}", tmp).unwrap();
        list.insert(tmp);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId {
    id: Option<Arc<str>>,
}

pub struct Timer {
    start: std::time::Instant,
}
impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    pub fn since(&self) -> Duration {
        self.start.elapsed()
    }
    pub fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        println!("took:{:#?}", self.since());
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectId {
    pub const fn is_valid(&self) -> bool {
        self.id.is_some()
    }

    pub const fn is_invalid(&self) -> bool {
        self.id.is_none()
    }

    pub const fn new_invalid() -> Self {
        Self { id: None }
    }

    pub fn new() -> Self {
        generate_id()
    }
}

pub struct SharedListInner<T> {
    list: VecDeque<T>,
    mutated: bool,
}

pub struct SharedList<T> {
    inner: Arc<std::sync::Mutex<SharedListInner<T>>>,
}

pub struct SharedListGuard<'a, T> {
    inner: std::sync::MutexGuard<'a, SharedListInner<T>>,
}

impl<'a, T> std::ops::Deref for SharedListGuard<'a, T> {
    type Target = VecDeque<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner.deref().list
    }
}
pub struct SharedListGuardMut<'a, T> {
    inner: std::sync::MutexGuard<'a, SharedListInner<T>>,
}
impl<'a, T> std::ops::Deref for SharedListGuardMut<'a, T> {
    type Target = VecDeque<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner.deref().list
    }
}

impl<'a, T> std::ops::DerefMut for SharedListGuardMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.deref_mut().list
    }
}
impl<T> Clone for SharedList<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Default for SharedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SharedList<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(SharedListInner {
                list: VecDeque::new(),
                mutated: true,
            })),
        }
    }

    pub fn lock_mut<'a>(&'a self) -> SharedListGuardMut<'a, T> {
        let mut tmp = match self.inner.lock() {
            Ok(x) => x,
            Err(x) => x.into_inner(),
        };
        tmp.mutated = true;
        SharedListGuardMut { inner: tmp }
    }

    pub fn lock<'a>(&'a self) -> SharedListGuard<'a, T> {
        let tmp = match self.inner.lock() {
            Ok(x) => x,
            Err(x) => x.into_inner(),
        };
        SharedListGuard { inner: tmp }
    }

    pub fn try_lock_mut<'a>(&'a self) -> Option<SharedListGuardMut<'a, T>> {
        let mut tmp = match self.inner.try_lock() {
            Ok(x) => x,
            Err(e) => match e {
                std::sync::TryLockError::Poisoned(x) => x.into_inner(),
                std::sync::TryLockError::WouldBlock => {
                    return None;
                }
            },
        };
        tmp.mutated = true;
        Some(SharedListGuardMut { inner: tmp })
    }

    pub fn try_lock<'a>(&'a self) -> Option<SharedListGuard<'a, T>> {
        let tmp = match self.inner.try_lock() {
            Ok(x) => x,
            Err(e) => match e {
                std::sync::TryLockError::Poisoned(x) => x.into_inner(),
                std::sync::TryLockError::WouldBlock => {
                    return None;
                }
            },
        };
        Some(SharedListGuard { inner: tmp })
    }

    pub fn push_front(&self, value: T) {
        self.lock_mut().push_front(value);
    }

    pub fn pop_front(&self) -> Option<T> {
        self.lock_mut().pop_front()
    }

    pub fn push_back(&self, value: T) {
        self.lock_mut().push_back(value);
    }

    pub fn pop_back(&self) -> Option<T> {
        self.lock_mut().pop_back()
    }

    pub fn take(&self, at: usize) -> Option<T> {
        self.lock_mut().remove(at)
    }

    pub fn replace(&self, at: usize, mut value: T) -> Result<T, T> {
        if let Some(x) = self.lock_mut().get_mut(at) {
            std::mem::swap(x, &mut value);
            Ok(value)
        } else {
            Err(value)
        }
    }

    pub fn set(&self, at: usize, value: T) {
        _ = self.replace(at, value);
    }

    pub fn insert(&self, at: usize, value: T) -> Result<(), T> {
        let mut guard = self.lock_mut();
        if guard.len() < at {
            Err(value)
        } else {
            guard.insert(at, value);
            Ok(())
        }
    }

    pub fn peek_mutated(&self) -> bool {
        let tmp = match self.inner.lock() {
            Ok(x) => x,
            Err(e) => e.into_inner(),
        };
        tmp.mutated
    }

    pub async fn consume_mutated(&self) -> bool {
        let mut tmp = match self.inner.lock() {
            Ok(x) => x,
            Err(e) => e.into_inner(),
        };
        let out = tmp.mutated;
        tmp.mutated = false;
        out
    }
    pub fn len(&self) -> usize {
        self.lock().len()
    }
}

impl<T: Clone> SharedList<T> {
    pub fn get(&self, at: usize) -> Option<T> {
        self.lock().get(at).cloned()
    }
}
#[derive(Serialize, Deserialize)]
pub struct Table<Key: Eq + Hash, Value> {
    table: std::sync::Mutex<HashMap<Key, Value>>,
}

impl<Key: Eq + Hash, Value> Default for Table<Key, Value> {
    fn default() -> Self {
        Self {
            table: std::sync::Mutex::new(HashMap::new()),
        }
    }
}
impl<
    Key: Eq + Hash + DeserializeOwned + Serialize + Clone,
    Value: Serialize + DeserializeOwned + Clone,
> Table<Key, Value>
{
    pub fn new() -> Self {
        Self {
            table: std::sync::Mutex::new(HashMap::new()),
        }
    }
    pub fn take_lock<'a>(&'a self) -> std::sync::MutexGuard<'a, HashMap<Key, Value>> {
        match self.table.try_lock() {
            Ok(x) => x,
            Err(x) => match x {
                std::sync::TryLockError::Poisoned(x) => x.into_inner(),
                std::sync::TryLockError::WouldBlock => {
                    panic!("would block")
                }
            },
        }
    }

    pub fn get(&self, key: &Key) -> Option<Value> {
        self.take_lock().get(key).map(|i| i.clone())
    }

    pub fn set(&self, key: Key, value: Value) {
        self.take_lock().insert(key, value);
    }

    pub fn select(&self, mut predicate: impl FnMut(&Key, &Value) -> bool) -> Vec<(Key, Value)> {
        let guard = self.take_lock();
        let mut out = Vec::new();
        for i in guard.iter() {
            if predicate(i.0, i.1) {
                out.push((i.0.clone(), i.1.clone()));
            }
        }
        out
    }

    pub fn select_by_keys(&self, mut predicate: impl FnMut(&Key) -> bool) -> Vec<(Key, Value)> {
        let guard = self.take_lock();
        let mut out = Vec::new();
        for i in guard.iter() {
            if predicate(i.0) {
                out.push((i.0.clone(), i.1.clone()));
            }
        }
        out
    }

    pub fn select_by_values(&self, mut predicate: impl FnMut(&Key) -> bool) -> Vec<(Key, Value)> {
        let guard = self.take_lock();
        let mut out = Vec::new();
        for i in guard.iter() {
            if predicate(i.0) {
                out.push((i.0.clone(), i.1.clone()));
            }
        }
        out
    }
    pub fn get_keys_matching(&self, mut predicate: impl FnMut(&Key, &Value) -> bool) -> Vec<Key> {
        let guard = self.take_lock();
        let mut out = Vec::new();
        for i in guard.iter() {
            if predicate(i.0, i.1) {
                out.push(i.0.clone());
            }
        }
        out
    }

    pub fn get_keys_by_matching_keys(&self, mut predicate: impl FnMut(&Key) -> bool) -> Vec<Key> {
        let guard = self.take_lock();
        let mut out = Vec::new();
        for i in guard.iter() {
            if predicate(i.0) {
                out.push(i.0.clone());
            }
        }
        out
    }

    pub fn get_keys_by_matching_values(&self, mut predicate: impl FnMut(&Key) -> bool) -> Vec<Key> {
        let guard = self.take_lock();
        let mut out = Vec::new();
        for i in guard.iter() {
            if predicate(i.0) {
                out.push(i.0.clone());
            }
        }
        out
    }

    pub fn store(&self, values: Vec<(Key, Value)>) {
        let mut guard = self.take_lock();
        for i in values {
            guard.insert(i.0, i.1);
        }
    }

    pub fn transform_matching(
        &self,
        mut predicate: impl FnMut(&Key, &Value) -> bool,
        mut transform: impl FnMut(&Key, &mut Value),
    ) {
        let mut guard = self.take_lock();
        for (key, value) in guard.iter_mut() {
            if predicate(key, value) {
                transform(key, value)
            }
        }
    }
    pub fn transform_matching_by_key(
        &self,
        mut predicate: impl FnMut(&Key) -> bool,
        mut transform: impl FnMut(&Key, &mut Value),
    ) {
        let mut guard = self.take_lock();
        for (key, value) in guard.iter_mut() {
            if predicate(key) {
                transform(key, value)
            }
        }
    }

    pub fn transform_matching_by_value(
        &self,
        mut predicate: impl FnMut(&Value) -> bool,
        mut transform: impl FnMut(&Key, &mut Value),
    ) {
        let mut guard = self.take_lock();
        for (key, value) in guard.iter_mut() {
            if predicate(value) {
                transform(key, value)
            }
        }
    }
}

impl<Value: Serialize + DeserializeOwned + Clone> Table<Arc<str>, Value> {
    pub fn load_from_folder(
        path: &str,
        extension: &str,
        mut custom: Option<&mut dyn FnMut(Arc<str>) -> Result<Value, std::io::Error>>,
    ) -> Result<Self, std::io::Error> {
        let dir = std::fs::read_dir(path)?;
        let out = Self::new();
        for i in dir {
            let tmp = i?;
            if tmp.file_type()?.is_file() {
                let pth = tmp.path();
                let mut should_load = false;
                let Some(name) = pth.file_name() else {
                    continue;
                };
                let Some(name) = name.to_str() else {
                    continue;
                };
                let extension = if let Some(e) = extension.strip_prefix(".") {
                    e
                } else {
                    extension
                };
                if let Some(ext) = pth.extension() {
                    if let Some(ext) = ext.to_str() {
                        if ext == extension {
                            should_load = true;
                        }
                    }
                } else {
                    if extension.is_empty() {
                        should_load = true;
                    }
                }
                if should_load {
                    let v = if let Some(func) = custom.as_mut() {
                        func(pth.to_str().unwrap().into())?
                    } else {
                        let s = std::fs::read(&pth)?;
                        let Ok(x) = rmp_serde::from_slice(&s) else {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "could not deserialize",
                            )
                            .into());
                        };
                        x
                    };
                    let k = name.into();
                    out.set(k, v);
                }
            }
        }
        Ok(out)
    }

    pub fn store_to_folder(
        &self,
        path: &str,
        extension: &str,
        mut custom: Option<&mut dyn FnMut(Arc<str>, &Value) -> Result<(), std::io::Error>>,
    ) -> Result<(), std::io::Error> {
        let g = self.take_lock();
        for (key, value) in g.iter() {
            let k = if let Some(t) = key.strip_suffix(extension) {
                t
            } else {
                key.as_ref()
            };
            let name = path.to_string() + "/" + &k + extension;
            if let Some(func) = custom.as_mut() {
                func(name.clone().into(), value).unwrap();
            } else {
                let Ok(v) = rmp_serde::to_vec(value) else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "could not serialize",
                    )
                    .into());
                };
                std::fs::write(name, v)?;
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PriorityQueue<T: PartialEq> {
    inner: VecDeque<T>,
}
impl<T: PartialEq> PriorityQueue<T> {
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
        }
    }

    pub fn next_value(&mut self) -> Option<T> {
        self.inner.pop_back()
    }

    pub fn next_value_rev(&mut self) -> Option<T> {
        self.inner.pop_front()
    }

    pub fn insert(&mut self, value: T) {
        let mut idx = 0;
        while idx < self.inner.len() {
            if self.inner[idx] == value {
                self.inner.remove(idx);
            } else {
                idx += 1;
            }
        }
        self.inner.push_front(value);
    }

    pub fn send_to_back(&mut self, value: T) {
        let mut idx = 0;
        while idx < self.inner.len() {
            if self.inner[idx] == value {
                self.inner.remove(idx);
            } else {
                idx += 1;
            }
        }
        self.inner.push_back(value);
    }
    pub fn remove(&mut self, value: &T) {
        let mut idx = 0;
        while idx < self.inner.len() {
            if self.inner[idx] == *value {
                self.inner.remove(idx);
            } else {
                idx += 1;
            }
        }
    }
}

pub struct Config<T: Serialize + DeserializeOwned + Default + 'static + Send + Sync> {
    inner: std::sync::Mutex<Option<T>>,
    create_func: &'static (dyn Fn(&'static str, &'static str, &mut T) + Send + Sync),
    save_func: &'static (dyn Fn(&'static str, &'static str, &mut T) + Send + Sync),
    file_name: &'static str,
    directory: &'static str,
    sub_folders: &'static [&'static str],
}
impl<T: Serialize + DeserializeOwned + Default + Send + Sync> Config<T> {
    pub const fn new(
        directory: &'static str,
        file_name: &'static str,
        create_func: &'static (dyn Fn(&'static str, &'static str, &mut T) + Send + Sync),
        save_func: &'static (dyn Fn(&'static str, &'static str, &mut T) + Send + Sync),
        sub_folders: &'static [&'static str],
    ) -> Self {
        Self {
            sub_folders,
            inner: std::sync::Mutex::new(None),
            create_func,
            save_func,
            file_name,
            directory,
        }
    }
    pub fn unsafe_mutable_inner_get<'a>(&'a self) -> std::sync::MutexGuard<'a, Option<T>> {
        let mut tmp = match self.inner.try_lock() {
            Ok(x) => x,
            Err(x) => match x {
                std::sync::TryLockError::Poisoned(x) => x.into_inner(),
                std::sync::TryLockError::WouldBlock => {
                    panic!("would block");
                }
            },
        };
        let mut errored = false;
        if tmp.is_none() {
            try_catch!(try {
                let path_to = self.directory.to_string() + "/" + self.file_name;
                let byte_buff = std::fs::read(&path_to)?;
                let base: T = serde_json::from_slice(&byte_buff)?;
                *tmp = Some(base);
            } catch |_x| {
                println!("caught");
                errored = true;
                if let Err(_) = std::fs::read_dir(self.directory){
                    std::fs::create_dir(self.directory).unwrap();
                    for i in self.sub_folders{
                        std::fs::create_dir(self.directory.to_string()+"/"+i).unwrap();
                    }
                }
                let value = T::default();
                *tmp = Some(value);
            });
            (self.create_func)(self.directory, self.file_name, tmp.as_mut().unwrap());
        }
        if errored {
            let value = tmp.as_mut().unwrap();
            let path_to = self.directory.to_string() + "/" + self.file_name;
            let v = serde_json::to_string(&value).unwrap();
            std::fs::write(path_to, v).unwrap();
            (self.save_func)(self.directory, self.file_name, value);
        }
        tmp
    }

    pub fn get(&'static self) -> ConfigGuard<T> {
        ConfigGuard {
            inner: self.unsafe_mutable_inner_get(),
        }
    }

    pub fn unadvised_get_mutable(&'static self) -> ConfigGuardMut<T> {
        ConfigGuardMut {
            inner: self.unsafe_mutable_inner_get(),
        }
    }

    pub fn save(&self) {
        let mut tmp = self.unsafe_mutable_inner_get();
        let value = tmp.as_mut().unwrap();
        (self.save_func)(self.directory, self.file_name, value);
        let path_to = self.directory.to_string() + "/" + self.file_name;
        let v = serde_json::to_string(&value).unwrap();
        std::fs::write(path_to, v).unwrap();
    }
}

pub struct ConfigGuard<T: Serialize + DeserializeOwned + Default + 'static> {
    inner: std::sync::MutexGuard<'static, Option<T>>,
}
impl<T: Serialize + DeserializeOwned + Default + 'static> Deref for ConfigGuard<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.inner.deref().as_ref().unwrap()
    }
}
pub struct ConfigGuardMut<T: Serialize + DeserializeOwned + Default + 'static> {
    inner: std::sync::MutexGuard<'static, Option<T>>,
}
impl<T: Serialize + DeserializeOwned + Default + 'static> Deref for ConfigGuardMut<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.inner.deref().as_ref().unwrap()
    }
}
impl<T: Serialize + DeserializeOwned + Default + 'static> DerefMut for ConfigGuardMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut().as_mut().unwrap()
    }
}

#[macro_export]
macro_rules! try_catch {
    (try $block:block catch |$value:ident| $catch:block) => {{
        let mut _func = (|| {$block Result::<(), Box<dyn std::error::Error>>::Ok(())});
        if let Err($value) = _func(){
            $catch
        }
    }};
}

pub struct HeapValue<T> {
    ptr: Option<Arc<tokio::sync::Mutex<T>>>,
    generation: u64,
}
pub struct HeapInner<T> {
    values: tokio::sync::Mutex<Vec<HeapValue<T>>>,
}

pub struct HeapRef<T> {
    parent: Weak<HeapInner<T>>,
    index: u64,
    generation: u64,
}
impl<T> Clone for HeapRef<T> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            index: self.index,
            generation: self.generation,
        }
    }
}
pub struct HeapRefGuard<'a, T> {
    value: tokio::sync::MutexGuard<'a, T>,
    _guard: std::cell::UnsafeCell<Option<Arc<Mutex<T>>>>,
}

impl<'a, T> std::ops::Deref for HeapRefGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value.deref()
    }
}

impl<'a, T> std::ops::DerefMut for HeapRefGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.deref_mut()
    }
}

pub struct Heap<T> {
    ptr: Arc<HeapInner<T>>,
}
impl<T> Heap<T> {
    pub fn new() -> Self {
        Self {
            ptr: Arc::new(HeapInner {
                values: Mutex::new(Vec::new()),
            }),
        }
    }

    pub async fn alloc(&self, value: T) -> HeapRef<T> {
        let mut guard = self.ptr.values.lock().await;
        for i in 0..guard.len() {
            if guard[i].ptr.is_none() {
                guard[i].generation = guard[i].generation.wrapping_add(1);
                guard[i].ptr = Some(Arc::new(Mutex::new(value)));
                return HeapRef {
                    parent: Arc::downgrade(&self.ptr),
                    index: i as u64,
                    generation: guard[i].generation,
                };
            }
        }
        let value = HeapValue {
            ptr: Some(Arc::new(Mutex::new(value))),
            generation: 1,
        };
        let idx = guard.len() as usize;
        guard.push(value);
        HeapRef {
            parent: Arc::downgrade(&self.ptr),
            index: idx as u64,
            generation: 1,
        }
    }

    pub async fn free(&self, ptr: HeapRef<T>) {
        let mut g = self.ptr.values.lock().await;
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.ptr.is_some() && x.generation == ptr.generation {
                x.ptr = None;
            }
        }
    }

    pub async fn realloc(&self, ptr: &HeapRef<T>, value: T) {
        let mut g = self.ptr.values.lock().await;
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.generation == ptr.generation {
                x.ptr = Some(Arc::new(Mutex::new(value)));
            }
        }
    }

    pub async fn is_valid_ptr(&self, ptr: &HeapRef<T>) -> bool {
        let mut g = self.ptr.values.lock().await;
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.generation == ptr.generation {
                return true;
            }
        }
        false
    }

    pub fn downgrade(&self) -> WeakHeap<T> {
        WeakHeap {
            ptr: Arc::downgrade(&self.ptr),
        }
    }

    pub fn alloc_blocking(&self, value: T) -> HeapRef<T> {
        let mut guard = self.ptr.values.blocking_lock();
        for i in 0..guard.len() {
            if guard[i].ptr.is_none() {
                guard[i].generation = guard[i].generation.wrapping_add(1);
                guard[i].ptr = Some(Arc::new(Mutex::new(value)));
                return HeapRef {
                    parent: Arc::downgrade(&self.ptr),
                    index: i as u64,
                    generation: guard[i].generation,
                };
            }
        }
        let value = HeapValue {
            ptr: Some(Arc::new(Mutex::new(value))),
            generation: 1,
        };
        let idx = guard.len() as usize;
        guard.push(value);
        HeapRef {
            parent: Arc::downgrade(&self.ptr),
            index: idx as u64,
            generation: 1,
        }
    }

    pub fn free_blocking(&self, ptr: HeapRef<T>) {
        let mut g = self.ptr.values.blocking_lock();
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.ptr.is_some() && x.generation == ptr.generation {
                x.ptr = None;
            }
        }
    }

    pub fn realloc_blocking(&self, ptr: &HeapRef<T>, value: T) {
        let mut g = self.ptr.values.blocking_lock();
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.generation == ptr.generation {
                x.ptr = Some(Arc::new(Mutex::new(value)));
            }
        }
    }

    pub fn is_valid_ptr_blocking(&self, ptr: &HeapRef<T>) -> bool {
        let mut g = self.ptr.values.blocking_lock();
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.generation == ptr.generation {
                return true;
            }
        }
        false
    }
}
impl<T> Clone for Heap<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone(),
        }
    }
}

impl<T> HeapRef<T> {
    pub const fn new() -> Self {
        Self {
            parent: Weak::new(),
            index: 0,
            generation: 0,
        }
    }

    pub async fn is_valid(&self) -> bool {
        let Some(parent) = self.parent.upgrade() else {
            return false;
        };
        let mut g = parent.values.lock().await;
        if let Some(x) = g.get_mut(self.index as usize) {
            if x.generation == self.generation {
                return true;
            }
        }
        false
    }

    pub fn get_heap(&self) -> Option<Heap<T>> {
        Some(Heap {
            ptr: self.parent.upgrade()?,
        })
    }
    pub async fn try_lock<'a>(&'a mut self) -> Option<HeapRefGuard<'a, T>> {
        if let Some(x) = self.parent.upgrade() {
            let v = x.values.lock().await;
            let g = v.get(self.index as usize)?;
            if g.generation != self.generation {
                return None;
            }
            let g2 = g.ptr.clone();

            let y = unsafe { std::mem::transmute(g2.as_ref()?.lock().await) };
            Some(HeapRefGuard {
                value: y,
                _guard: std::cell::UnsafeCell::new(g2),
            })
        } else {
            None
        }
    }

    pub async fn lock<'a>(&'a mut self) -> HeapRefGuard<'a, T> {
        self.try_lock().await.unwrap()
    }

    pub fn try_lock_blocking<'a>(&'a mut self) -> Option<HeapRefGuard<'a, T>> {
        if let Some(x) = self.parent.upgrade() {
            let v = x.values.blocking_lock();
            let g = v.get(self.index as usize)?;
            if g.generation != self.generation {
                return None;
            }
            let mut g2 = g.ptr.clone();
            let y = unsafe { std::mem::transmute(g2.as_mut().unwrap().blocking_lock()) };
            Some(HeapRefGuard {
                value: y,
                _guard: std::cell::UnsafeCell::new(g2),
            })
        } else {
            None
        }
    }

    pub fn lock_blocking<'a>(&'a mut self) -> HeapRefGuard<'a, T> {
        self.try_lock_blocking().unwrap()
    }
}

impl<T> Default for HeapRef<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PartialEq for HeapRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation
            && self.index == other.index
            && self.parent.ptr_eq(&other.parent)
    }
}
impl<T> Eq for HeapRef<T> {}
impl<T> Hash for HeapRef<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.parent.as_ptr() as usize);
        state.write_u64(self.index);
        state.write_u64(self.generation);
    }
}

pub struct WeakHeap<T> {
    ptr: Weak<HeapInner<T>>,
}
impl<T> Clone for WeakHeap<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone(),
        }
    }
}
impl<T> WeakHeap<T> {
    pub fn new() -> Self {
        Self { ptr: Weak::new() }
    }

    pub async fn try_alloc(&self, value: T) -> Option<HeapRef<T>> {
        let heap = self.ptr.upgrade()?;
        let mut guard = heap.values.lock().await;
        for i in 0..guard.len() {
            if guard[i].ptr.is_none() {
                guard[i].generation = guard[i].generation.wrapping_add(1);
                guard[i].ptr = Some(Arc::new(Mutex::new(value)));
                return Some(HeapRef {
                    parent: self.ptr.clone(),
                    index: i as u64,
                    generation: guard[i].generation,
                });
            }
        }
        let value = HeapValue {
            ptr: Some(Arc::new(Mutex::new(value))),
            generation: 1,
        };
        let idx = guard.len() as usize;
        guard.push(value);
        Some(HeapRef {
            parent: self.ptr.clone(),
            index: idx as u64,
            generation: 1,
        })
    }

    pub async fn try_free(&self, ptr: HeapRef<T>) -> Option<()> {
        let heap = self.ptr.upgrade()?;
        let mut g = heap.values.lock().await;
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.ptr.is_some() && x.generation == ptr.generation {
                x.ptr = None;
            }
        }
        Some(())
    }

    pub async fn try_realloc(&self, ptr: &HeapRef<T>, value: T) -> Option<()> {
        let heap = self.ptr.upgrade()?;
        let mut g = heap.values.lock().await;
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.generation == ptr.generation {
                x.ptr = Some(Arc::new(Mutex::new(value)));
            }
        }
        Some(())
    }

    pub async fn is_valid_ptr(&self, ptr: &HeapRef<T>) -> bool {
        let Some(heap) = self.ptr.upgrade() else {
            return false;
        };
        let mut g = heap.values.lock().await;
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.generation == ptr.generation {
                return true;
            }
        }
        false
    }

    pub async fn alloc(&self, value: T) -> HeapRef<T> {
        self.try_alloc(value).await.unwrap()
    }

    pub async fn free(&self, ptr: HeapRef<T>) {
        let _ = self.try_free(ptr).await;
    }

    pub async fn realloc(&self, ptr: &HeapRef<T>, value: T) {
        let _ = self.try_realloc(ptr, value).await;
    }

    pub fn upgrade(&self) -> Option<Heap<T>> {
        Some(Heap {
            ptr: self.ptr.upgrade()?,
        })
    }

    pub fn try_alloc_blocking(&self, value: T) -> Option<HeapRef<T>> {
        let heap = self.ptr.upgrade()?;
        let mut guard = heap.values.blocking_lock();
        for i in 0..guard.len() {
            if guard[i].ptr.is_none() {
                guard[i].generation = guard[i].generation.wrapping_add(1);
                guard[i].ptr = Some(Arc::new(Mutex::new(value)));
                return Some(HeapRef {
                    parent: self.ptr.clone(),
                    index: i as u64,
                    generation: guard[i].generation,
                });
            }
        }
        let value = HeapValue {
            ptr: Some(Arc::new(Mutex::new(value))),
            generation: 1,
        };
        let idx = guard.len() as usize;
        guard.push(value);
        Some(HeapRef {
            parent: self.ptr.clone(),
            index: idx as u64,
            generation: 1,
        })
    }

    pub fn try_free_blocking(&self, ptr: HeapRef<T>) -> Option<()> {
        let heap = self.ptr.upgrade()?;
        let mut g = heap.values.blocking_lock();
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.ptr.is_some() && x.generation == ptr.generation {
                x.ptr = None;
            }
        }
        Some(())
    }

    pub fn try_realloc_blocking(&self, ptr: &HeapRef<T>, value: T) -> Option<()> {
        let heap = self.ptr.upgrade()?;
        let mut g = heap.values.blocking_lock();
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.generation == ptr.generation {
                x.ptr = Some(Arc::new(Mutex::new(value)));
            }
        }
        Some(())
    }

    pub fn is_valid_ptr_blocking(&self, ptr: &HeapRef<T>) -> bool {
        let Some(heap) = self.ptr.upgrade() else {
            return false;
        };
        let mut g = heap.values.blocking_lock();
        if let Some(x) = g.get_mut(ptr.index as usize) {
            if x.generation == ptr.generation {
                return true;
            }
        }
        false
    }

    pub fn alloc_blocking(&self, value: T) -> HeapRef<T> {
        self.try_alloc_blocking(value).unwrap()
    }

    pub fn free_blocking(&self, ptr: HeapRef<T>) {
        let _ = self.try_free_blocking(ptr);
    }

    pub fn realloc_blocking(&self, ptr: &HeapRef<T>, value: T) {
        let _ = self.try_realloc_blocking(ptr, value);
    }
}

pub struct StaticHeap<T> {
    inner: tokio::sync::Mutex<Option<Heap<T>>>,
}
impl<T> StaticHeap<T> {
    pub const fn new() -> Self {
        Self {
            inner: tokio::sync::Mutex::const_new(None),
        }
    }

    pub async fn get(&'static self) -> MutexGuard<'static, Option<Heap<T>>> {
        let mut tmp = self.inner.lock().await;
        if tmp.is_none() {
            *tmp = Some(Heap::new())
        }
        tmp
    }

    pub fn get_blocking(&'static self) -> MutexGuard<'static, Option<Heap<T>>> {
        let mut tmp = self.inner.blocking_lock();
        if tmp.is_none() {
            *tmp = Some(Heap::new())
        }
        tmp
    }

    pub async fn alloc(&'static self, value: T) -> HeapRef<T> {
        self.get().await.as_mut().unwrap().alloc(value).await
    }

    pub async fn free(&'static self, ptr: HeapRef<T>) {
        self.get().await.as_mut().unwrap().free(ptr).await
    }

    pub async fn realloc(&'static self, ptr: &HeapRef<T>, value: T) {
        self.get().await.as_mut().unwrap().realloc(ptr, value).await
    }

    pub async fn is_valid_ptr(&'static self, ptr: &HeapRef<T>) -> bool {
        self.get().await.as_mut().unwrap().is_valid_ptr(ptr).await
    }

    pub fn downgrade(&'static self) -> WeakHeap<T> {
        self.get_blocking().as_mut().unwrap().downgrade()
    }

    pub fn as_heap(&'static self) -> Heap<T> {
        self.get_blocking().as_mut().unwrap().clone()
    }
    pub fn alloc_blocking(&'static self, value: T) -> HeapRef<T> {
        self.get_blocking().as_mut().unwrap().alloc_blocking(value)
    }

    pub fn free_blocking(&'static self, ptr: HeapRef<T>) {
        self.get_blocking().as_mut().unwrap().free_blocking(ptr)
    }

    pub fn realloc_blocking(&'static self, ptr: &HeapRef<T>, value: T) {
        self.get_blocking()
            .as_mut()
            .unwrap()
            .realloc_blocking(ptr, value);
    }

    pub fn is_valid_ptr_blocking(&'static self, ptr: &HeapRef<T>) -> bool {
        self.get_blocking()
            .as_mut()
            .unwrap()
            .is_valid_ptr_blocking(ptr)
    }
}

pub struct HeapIterator<'a, T> {
    idx: usize,
    rf: &'a Arc<HeapInner<T>>,
}
impl<'a, T> Iterator for HeapIterator<'a, T> {
    type Item = HeapRef<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let guard = self.rf.values.blocking_lock();
        while self.idx < guard.len() {
            if guard[self.idx].ptr.is_some() {
                let v = guard[self.idx].generation;
                let out = HeapRef {
                    parent: Arc::downgrade(&self.rf),
                    index: self.idx as u64,
                    generation: v,
                };
                self.idx += 1;
                return Some(out);
            } else {
                self.idx += 1;
            }
        }
        None
    }
}
