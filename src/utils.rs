use std::{collections::VecDeque, marker::PhantomData, sync::Arc, task::Waker};

use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

pub struct Stream<T: Serialize + DeserializeOwned> {
    data: PhantomData<T>,
    stream: Mutex<TcpStream>,
}
impl<T: DeserializeOwned + Serialize> Stream<T> {
    pub fn new(st: TcpStream) -> Self {
        Self {
            data: Default::default(),
            stream: Mutex::new(st),
        }
    }

    pub async fn send(&self, value: &T) -> Result<usize, tokio::io::Error> {
        let v = serde_json::to_vec(value).unwrap();
        let count = v.len();
        let mut guard = self.stream.lock().await;
        guard.write_u64_le(count as u64).await?;
        guard.write_all(&v).await?;
        Ok(count)
    }

    pub fn send_blocking(&self, value: &T) -> Result<usize, tokio::io::Error> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(self.send(value))
    }

    pub async fn receive(&self) -> Result<T, tokio::io::Error> {
        let mut guard = self.stream.lock().await;
        let count = guard.read_u64_le().await? as usize;
        let mut buffer = vec![0u8; count];
        guard.read_exact(&mut buffer).await?;
        let value = serde_json::from_slice(&buffer);
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
        let g = guard.peek(&mut count).await;
        match g {
            Ok(x) => {
                if x == 8 {
                    let count = guard.read_u64_le().await?;
                    let mut v = vec![0u8; count as usize];
                    guard.read_exact(&mut v).await?;
                    let out = serde_json::from_slice(&v)?;
                    Ok(Some(out))
                } else {
                    Ok(None)
                }
            }
            Err(x) => match x.kind() {
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
        match self.try_receive_blocking() {
            Ok(x) => x,
            Err(_) => None,
        }
    }
}
