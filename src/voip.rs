//-framework AudioToolbox -framework AudioUnit -framework IOKit -framework CoreAudio -framework OpenAL
use cpal::{
    Device, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::max,
    collections::{HashSet, VecDeque},
    io::ErrorKind,
    mem::size_of,
    net::UdpSocket,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant, UNIX_EPOCH},
};

use crate::{
    id::{ArachneId, GlobalId},
    rtils::marathon::BStream,
};
pub const RATE: u32 = 44100;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum VoipCmd {
    Mute,
    UnMute,
    Deafen,
    UnDeafen,
    Exit,
}

pub struct VoipClient {
    pub id: GlobalId,
    pub cmds: BStream<VoipCmd>,
    pub con: Arc<Mutex<UdpSocket>>,
}

pub fn spawn_voip(con: &str, id: GlobalId) -> Result<BStream<VoipCmd>, ()> {
    let (con1, con2) = BStream::create();
    let sock;
    let mut idx = 4096;
    loop {
        if let Ok(e) = UdpSocket::bind(("127.0.0.1", idx)) {
            sock = e;
            break;
        } else {
            idx += 1;
        }
        if idx >= u16::MAX - 1 {
            return Err(());
        }
    }
    let Ok(_) = sock.connect((con, 8009)) else {
        return Err(());
    };
    sock.set_nonblocking(true).unwrap();
    let mut client = VoipClient {
        cmds: con1,
        con: Arc::new(Mutex::new(sock)),
        id,
    };
    std::thread::spawn(move || client.run());
    Ok(con2)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Packet {
    time: u128,
    bytes: [i16; 724],
    from: GlobalId,
    counter: u64,
    count: u64,
}
#[derive(Debug, Clone)]
pub struct AudIoInner {
    line: VecDeque<Packet>,
    last_sent: u64,
    last_from: GlobalId,
    last_time: Instant,
}
impl Default for AudIoInner {
    fn default() -> Self {
        Self::new()
    }
}

impl AudIoInner {
    pub fn poll(&mut self) -> Option<Packet> {
        let tmp = self.line.pop_front()?;
        if tmp.from == self.last_from {
            if tmp.counter != (self.last_sent + 1) % 4_000_000
                && self.last_time.elapsed().as_millis() < 1
            {
                self.line.push_front(tmp);
                return None;
            }
            self.last_from = tmp.from;
            self.last_sent = tmp.counter;
            self.last_time = Instant::now();
            Some(tmp)
        } else {
            if self.last_time.elapsed().as_millis() < 1 {
                self.line.push_front(tmp);
                let l = self.line.len();
                for _ in 0..l {
                    if self.line[0].time > UNIX_EPOCH.elapsed().unwrap().as_millis() + 10 {
                        self.line.pop_front();
                    }
                }
                None
            } else {
                let l = self.line.len();
                for _ in 0..l {
                    if self.line[0].time > UNIX_EPOCH.elapsed().unwrap().as_millis() + 10 {
                        self.line.pop_front();
                    }
                }
                self.last_from = tmp.from;
                self.last_sent = tmp.counter;
                self.last_time = Instant::now();
                Some(tmp)
            }
        }
    }

    pub fn enter(&mut self, value: Packet) {
        let now = UNIX_EPOCH.elapsed().unwrap().as_millis();
        if value.time.abs_diff(now) > 2000 {
            return;
        }
        if self.line.len() > 16 {
            let l = self.line.len();
            for _ in 0..l - 16 {
                self.line.pop_front();
            }
        }
        let mut to_enter = -1;
        for (idx, _) in self.line.iter().enumerate() {
            let p = self.line[idx];
            if p.time > value.time {
                if p.time.abs_diff(value.time) < 50 {
                    if value.from != p.from {
                        let x = &mut self.line[idx];
                        for i in 0..x.bytes.len() {
                            x.bytes[i] = if p.bytes[i].abs() > x.bytes[i].abs() {
                                p.bytes[i]
                            } else {
                                x.bytes[i]
                            };
                        }
                        x.count = max(x.count, p.count);
                        return;
                    }
                } else {
                    to_enter = idx as i32;
                    break;
                }
            }
        }
        if to_enter == -1 {
            self.line.push_back(value);
        } else {
            if to_enter == 0 {
                self.line.push_front(value);
            } else {
                self.line.insert((to_enter + 1) as usize, value);
            }
        }
    }

    pub fn new() -> Self {
        Self {
            line: VecDeque::new(),
            last_sent: 0,
            last_from: GlobalId::invalid(),
            last_time: Instant::now(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct AudIo {
    inner: Arc<Mutex<AudIoInner>>,
}

impl Default for AudIo {
    fn default() -> Self {
        Self::new()
    }
}

impl AudIo {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AudIoInner::new())),
        }
    }
    pub fn poll(&self) -> Option<Packet> {
        self.inner.lock().unwrap().poll()
    }
    pub fn enter(&self, packet: Packet) {
        self.inner.lock().unwrap().enter(packet);
    }
}
pub fn viable_input_config(
    device: &Device,
    in_muted: Arc<RwLock<bool>>,
    sock: Arc<Mutex<UdpSocket>>,
    id: GlobalId,
) -> Option<Stream> {
    let mut out = None;
    let configs = device.supported_input_configs().unwrap();
    for i in configs {
        println!("{}", i.min_sample_rate());
        let t = i.with_sample_rate(RATE);
        {
            let in_muted = in_muted.clone();
            let sock = sock.clone();
            let counter = Arc::new(Mutex::new(0));
            let tmp = device.build_input_stream(
                &t.config(),
                move |stream: &[f32], _info| {
                    // println!("{:#?}", stream);
                    let read = in_muted.read().unwrap();
                    if *read {
                        return;
                    }
                    let sock = sock.lock().unwrap();
                    let mut count = counter.lock().unwrap();
                    let mut idx = 0;
                    let mut pack = Packet {
                        from: id,
                        counter: *count,
                        time: UNIX_EPOCH.elapsed().unwrap().as_millis(),
                        bytes: [0; _],
                        count: 0,
                    };
                    *count += 1;
                    *count %= 4_000_000;
                    let mut not_zero = true;
                    while idx < stream.len() {
                        pack.bytes[pack.count as usize] =
                            (stream[idx] * (i16::MAX as f32 * 0.9)) as i16;
                        if stream[idx].abs() >= 0.01 {
                            not_zero = true;
                        }
                        pack.count += 1;
                        idx += 1;
                        if pack.count >= pack.bytes.len() as u64 {
                            if not_zero {
                                let bytes: [u8; size_of::<Packet>()] =
                                    unsafe { std::mem::transmute_copy(&pack) };
                                sock.set_nonblocking(false).unwrap();
                                sock.send(&bytes).unwrap();
                                sock.set_nonblocking(true).unwrap();
                            }
                            not_zero = false;
                            pack.count = 0;
                        }
                    }
                    if pack.count != 0 {
                        let bytes: [u8; size_of::<Packet>()] =
                            unsafe { std::mem::transmute_copy(&pack) };
                        sock.set_nonblocking(false).unwrap();
                        sock.send(&bytes).unwrap();
                        sock.set_nonblocking(true).unwrap();
                    }
                },
                |_err| todo!(),
                Some(Duration::from_micros(500)),
            );
            match tmp {
                Ok(x) => {
                    out = Some(x);
                    break;
                }
                Err(e) => {
                    println!("{:#?}", e);
                }
            }
        }
    }
    out
}
pub fn viable_output_config(
    device: &Device,
    in_deafend: Arc<RwLock<bool>>,
    id: GlobalId,
    input: AudIo,
) -> Option<Stream> {
    let mut out = None;
    let configs = device.supported_output_configs().unwrap();
    for i in configs {
        let t = i.with_sample_rate(RATE);
        {
            let in_deafend = in_deafend.clone();
            let input = input.clone();
            let tmp = device.build_output_stream(
                &t.config(),
                move |cb: &mut [f32], _out| {
                    let Ok(read) = in_deafend.read() else {
                        return;
                    };
                    if *read {
                        return;
                    }
                    let mut idx = 0;
                    while idx < cb.len() {
                        let f = input.poll();
                        let Some(pack) = f else {
                            break;
                        };
                        //  println!("packet:{:#?}", pack.from);
                        if pack.from == id {
                            continue;
                        }
                        for i in 0..pack.count {
                            if idx + i as usize >= cb.len() {
                                break;
                            }
                            cb[idx + i as usize] =
                                pack.bytes[i as usize] as f32 / (i16::MAX as f32 * 0.9);
                        }
                        idx += pack.count as usize;
                    }
                },
                |_er| {},
                Some(Duration::from_micros(500)),
            );
            match tmp {
                Ok(x) => {
                    out = Some(x);
                    break;
                }
                Err(e) => {
                    println!("{:#?}", e);
                }
            }
        }
    }
    out
}
impl VoipClient {
    pub fn run(&mut self) {
        let muted = Arc::new(RwLock::new(true));
        let deafened = Arc::new(RwLock::new(false));
        let host = cpal::default_host();
        let input_device = host.default_input_device().unwrap();
        // println!("input:{:#?}", input_device.description().unwrap());
        let output_device = host.default_output_device().unwrap();
        //println!("output:{:#?}", output_device.description().unwrap());
        let in_sock = self.con.clone();
        let in_muted = muted.clone();
        let inputs = AudIo::new();
        let output =
            viable_output_config(&output_device, deafened.clone(), self.id, inputs.clone())
                .unwrap();
        let input = viable_input_config(&input_device, in_muted, in_sock, self.id).unwrap();
        input.play().unwrap();
        output.play().unwrap();
        {
            let con = self.con.lock().unwrap();
            con.set_nonblocking(false).unwrap();
            con.send(b"toast is awesome").unwrap();
            con.set_nonblocking(true).unwrap();
        }
        let mut bytes: [u8; size_of::<Packet>()] = [0; _];
        'outer: loop {
            while let Ok(Some(cmd)) = self.cmds.receive() {
                match cmd {
                    VoipCmd::Exit => {
                        break 'outer;
                    }
                    VoipCmd::Deafen => {
                        let mut lock = deafened.write().unwrap();
                        *lock = true;
                    }
                    VoipCmd::Mute => {
                        let mut lock = muted.write().unwrap();
                        *lock = true;
                    }
                    VoipCmd::UnDeafen => {
                        let mut lock = deafened.write().unwrap();
                        *lock = false;
                    }
                    VoipCmd::UnMute => {
                        let mut lock = muted.write().unwrap();
                        println!("unmuted");
                        *lock = false;
                    }
                }
            }
            loop {
                let lock = self.con.lock().unwrap();
                lock.set_nonblocking(true).unwrap();
                let e = lock.recv(&mut bytes);
                match e {
                    Ok(x) => {
                        if x == size_of::<Packet>() {
                            inputs.enter(unsafe { std::mem::transmute_copy(&bytes) });
                        } else {
                            continue;
                        }
                    }
                    Err(e) => match e.kind() {
                        ErrorKind::WouldBlock => {
                            break;
                        }
                        _ => {
                            println!("{}", e);
                            todo!()
                        }
                    },
                }
            }
        }
        output.pause().unwrap();
        input.pause().unwrap();
    }
}

pub struct VoipServer {
    pub sock: UdpSocket,
    pub killer: BStream<()>,
}
pub fn spawn_voip_server(port: String) -> BStream<()> {
    let (kl, out) = BStream::create();
    println!("port:{:#?}", port);
    let sock = UdpSocket::bind((port, 8009)).unwrap();
    let mut server = VoipServer { sock, killer: kl };
    std::thread::spawn(move || {
        server.run();
    });
    out
}

impl VoipServer {
    pub fn run(&mut self) {
        let mut buf = [0; std::mem::size_of::<Packet>()];
        // self.sock.set_broadcast(true).unwrap();
        self.sock.set_nonblocking(true).unwrap();
        self.sock.set_broadcast(true).unwrap();
        let mut con_list = HashSet::new();
        let mut to_rem = Vec::new();
        let mut packets: Vec<Packet> = Vec::new();
        let mut tmps = Vec::new();
        'outer: loop {
            let e = self.killer.receive();
            if e.is_err() {
                break;
            }
            if let Ok(x) = e
                && x.is_some()
            {
                break;
            }
            let dt = Instant::now();
            loop {
                let Ok((count, addr)) = self.sock.recv_from(&mut buf) else {
                    break;
                };
                if !con_list.contains(&addr) {
                    println!("{} connected", addr);
                    con_list.insert(addr);
                }
                if count != std::mem::size_of::<Packet>() {
                    continue;
                }
                packets.push(unsafe { std::mem::transmute(buf) });
                if dt.elapsed().as_micros() > 100 || packets.len() > 100 {
                    break;
                }
            }
            if packets.is_empty() {
                continue 'outer;
            }
            packets.sort_by(|i, j| i.time.cmp(&j.time));
            for i in packets.drain(0..packets.len()) {
                tmps.push(i);
            }
            self.sock.set_nonblocking(false).unwrap();
            for j in tmps.drain(0..tmps.len()) {
                let buf: [u8; size_of::<Packet>()] = unsafe { std::mem::transmute(j) };
                for i in &con_list {
                    let e = self.sock.send_to(&buf, i);
                    if let Err(e) = e {
                        match e.kind() {
                            ErrorKind::WouldBlock => {
                                continue 'outer;
                            }
                            _ => {
                                to_rem.push(*i);
                            }
                        }
                    } else {
                        //     println!("sent to:{}", i)
                    }
                }
            }
            for i in &to_rem {
                println!("removed {}", i);
                con_list.remove(i);
            }
            to_rem.clear();
        }
    }
}
