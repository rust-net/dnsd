use std::collections::HashMap;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;

const LISTEN: &str = "127.0.0.1:53";
const SERVER: &str = "1.1.1.1:53";

struct DNS<'a> {
    value: &'a [u8],
    offset: usize,
}
impl<'a> DNS<'a> {
    pub fn with(value: &'a [u8], offset: usize) -> Self {
        return Self {
            value: value,
            offset: offset,
        };
    }
    pub fn to_string(&self) -> String {
        let mut str = String::with_capacity(1024);
        for a in &self.value[2..] {
            // str.push_str(format!("{:08b} ", a).as_str());
            str.push_str(format!("{:02x} ", a).as_str());
        }
        str.pop();
        str
    }
    pub fn id(&self) -> u16 {
        self.value[self.offset + 0] as u16 * 256 + self.value[self.offset + 1] as u16
    }
    pub fn qr(&self) -> &'static str {
        let qr: u8 = self.value[self.offset + 2] >> 7;
        if qr == 0 {
            "request"
        } else {
            "response"
        }
    }
    pub fn opcode(&self) -> &'static str {
        let opcode: u8 = (self.value[self.offset + 2] & 0b_0_1111_000) >> 3;
        match opcode {
            0 => "标准查询",
            1 => "反转查询",
            2 => "状态查询",
            _ => "保留",
        }
    }
    pub fn rcode(&self) -> &'static str {
        let opcode: u8 = self.value[self.offset + 3] & 0b_0000_1111;
        match opcode {
            0 => "没有错误",
            1 => "请求格式有误，服务器无法解析请求",
            2 => "服务器出错",
            3 => "请求中的域名不存在",
            4 => "服务器不支持该请求类型",
            5 => "服务器拒绝执行请求操作",
            _ => "保留",
        }
    }
    pub fn qdcount(&self) -> u16 {
        let count: u16 =
            self.value[self.offset + 4] as u16 * 256 + self.value[self.offset + 5] as u16;
        count
    }
    pub fn ancount(&self) -> u16 {
        let count: u16 =
            self.value[self.offset + 6] as u16 * 256 + self.value[self.offset + 7] as u16;
        count
    }
    pub fn nscount(&self) -> u16 {
        let count: u16 =
            self.value[self.offset + 8] as u16 * 256 + self.value[self.offset + 9] as u16;
        count
    }
    pub fn arcount(&self) -> u16 {
        let count: u16 =
            self.value[self.offset + 10] as u16 * 256 + self.value[self.offset + 11] as u16;
        count
    }
    pub fn question_list(&self) -> Vec<String> {
        let mut vec = Vec::with_capacity(1);
        for i in 0..self.qdcount() {
            let mut str = String::with_capacity(1024);
            let mut j = 12;
            loop {
                let ch = self.value[self.offset + j];
                j += 1;
                for k in 0..ch {
                    let nch = self.value[self.offset + j + k as usize] as char;
                    str.push(nch);
                }
                j += ch as usize;
                if self.value[self.offset + j] == 0 {
                    break;
                }
                str.push('.');
            }
            vec.push(str);
        }
        vec
    }
    pub fn question(&self) -> String {
        let mut str = String::with_capacity(1024);
        str.push_str("{ ");
        for name in self.question_list() {
            str.push_str(&format!("{} ", name));
        }
        str.push_str("}");
        str
    }
    pub fn info(&self) {
        if self.qr() == "request" {
            println!(
                "{id} {opcode} {question}",
                opcode = self.opcode(),
                id = self.id(),
                question = self.question(),
            );
        } else {
            println!(
                "{id} {opcode} {question} ({rcode})",
                rcode = self.rcode(),
                opcode = self.opcode(),
                id = self.id(),
                question = self.question(),
            );
        }
        // println!("{:02x?}", self.value); // 以十六进制而非十进制打印数组
        println!("[{}]", self.to_string());
        println!(
            "-----------------------------------------------------------------------------------"
        );
    }
}
#[test]
fn test() {
    DNS::with(
        &[
            0u8, 0, 245, 178, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 6, 103, 111, 111, 103, 108, 101, 3, 99,
            111, 109, 0, 0, 1, 0, 1,
        ],
        2,
    )
    .info();
    DNS::with(
        &[
            0u8, 44, 245, 178, 129, 128, 0, 1, 0, 1, 0, 0, 0, 0, 6, 103, 111, 111, 103, 108, 101,
            3, 99, 111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 176, 0, 4, 142, 0, 176,
            0, 4, 142, 250, 72, 174,
        ],
        2,
    )
    .info();
}

async fn udp_serv() -> std::io::Result<()> {
    let listen = if let Some(listen) = std::env::args().nth(1) {
        listen
    } else {
        LISTEN.to_string()
    };
    let log = if let Some(_) = std::env::args().nth(3) {
        true
    } else {
        // false
        true
    };
    let listener = tokio::net::UdpSocket::bind(listen).await;
    if let Err(e) = listener {
        println!("无法启用监听服务: {}", e);
        return Err(e);
    }
    let listener: UdpSocket = listener.ok().unwrap();
    let listener = std::sync::Arc::new(listener);
    println!("DNS代理服务已启动");

    let map: HashMap<_, _> = HashMap::<Vec<u8>, Vec<u8>>::with_capacity(1024); // cache
    let map: tokio::sync::Mutex<_> = tokio::sync::Mutex::new(map); // Mutex<HashMap>
    let map: std::sync::Arc<_> = std::sync::Arc::new(map); // Arc<Mutex<HashMap>>

    let mut query = [0u8; 1024]; // DNS query request data
    loop {
        let listener = listener.clone();
        let recv_result = listener.recv_from(&mut query[2..]).await;
        if let Err(_) = recv_result {
            continue;
        }
        let (received, client) = recv_result.ok().unwrap();
        if log {
            println!("==>>  DNS查询  ==>>");
            DNS::with(&query[..received + 2], 2).info();
        }

        let map1 = map.clone();
        let map2 = map.clone();
        // Find cache
        let cache = map1.lock().await;
        let cache = cache.get(&query[4..received + 2]);
        if let Some(cache) = cache {
            let mut buf = vec![0u8; 2 + cache.len()];
            buf[0] = query[2];
            buf[1] = query[3];
            &buf[2..2 + cache.len()].copy_from_slice(&cache[..]);
            if log {
                println!("                                      <<==  已缓存  <<==");
                print!("                                      ");
                DNS::with(&buf[..cache.len()], 0).info();
            }
            if let Err(e) = listener.send_to(&buf, client).await {
                eprintln!("Error: {}", e);
            }
            continue;
        }

        tokio::spawn(async move {
            // TCP data body length
            query[0] = (received / 0xff) as u8;
            query[1] = (received % 0xff) as u8;

            // Connect server
            let server = if let Some(server) = std::env::args().nth(2) {
                server
            } else {
                SERVER.to_string()
            };
            let tcp = tokio::net::TcpStream::connect(&server).await;
            if let Err(e) = tcp {
                println!("无法连接服务器{}：{}", &server, e);
                return;
            }
            let mut tcp = tcp.ok().unwrap();

            // Faword query request
            let writed = tcp.write(&query[..received + 2]).await;
            if let Err(e) = writed {
                println!("意外的错误：{}", e);
                return;
            }

            let mut resp = [0u8; 1024];
            if let Ok(le) = tcp.read(&mut resp).await {
                let mut map = map2.try_lock().unwrap();
                map.insert(Vec::from(&query[4..received + 2]), Vec::from(&resp[4..le]));
                listener.send_to(&resp[2..le], client).await.unwrap();
                if log {
                    println!("                                      <<==  DNS响应  <<==");
                    print!("                                      ");
                    DNS::with(&resp[..le], 2).info();
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    udp_serv().await
}
