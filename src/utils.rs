use std::{
    collections::VecDeque,
    marker::PhantomData,
    sync::{Arc, atomic::AtomicBool},
    task::Waker,
    time::Duration,
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

pub struct Stream<T: Serialize + DeserializeOwned> {
    data: PhantomData<T>,
    stream: Mutex<TcpStream>,
    has_failed_at_some_point: AtomicBool,
}
impl<T: DeserializeOwned + Serialize> Stream<T> {
    pub fn new(st: TcpStream) -> Self {
        Self {
            data: Default::default(),
            stream: Mutex::new(st),
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
            BStreamInner::Local(v) => v.has_other(),
            BStreamInner::Network(n) => n.has_errored_fatally(),
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
