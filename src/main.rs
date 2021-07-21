use std::collections::HashMap;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;

use colored::*;

const DEBUG: bool = true;

const LISTEN: &str = "127.0.0.1:53";
const SERVER: &str = "1.1.1.1:53";
// const SERVER: &str = "114.114.114.114:53";

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
    // pub fn nscount(&self) -> u16 {
    //     let count: u16 =
    //         self.value[self.offset + 8] as u16 * 256 + self.value[self.offset + 9] as u16;
    //     count
    // }
    // pub fn arcount(&self) -> u16 {
    //     let count: u16 =
    //         self.value[self.offset + 10] as u16 * 256 + self.value[self.offset + 11] as u16;
    //     count
    // }
    pub fn question_list(&self) -> Vec<(String, &'static str)> {
        let mut vec = Vec::with_capacity(1);
        let mut j = 12;
        for _ in 0..self.qdcount() {
            let mut str = String::with_capacity(1024);
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
            let qtype: u16 = self.value[self.offset + j + 1] as u16 * 256
                + self.value[self.offset + j + 2] as u16;
            vec.push((
                str,
                match qtype {
                    1 => "A",
                    0x1c => "AAAA",
                    2 => "NS",
                    3 => "MD",
                    4 => "MF",
                    5 => "CNAME",
                    15 => "MX",
                    16 => "TXT",
                    _ => "_",
                },
            ));
            j += 4; // QTYPE 2 bytes and QCLASS 2 bytes
        }
        vec
    }
    pub fn question(&self) -> String {
        let mut str = String::with_capacity(1024);
        str.push_str("{ ");
        for question in self.question_list() {
            str.push_str(&format!("{name} ({type}) ", name = question.0, type = question.1));
        }
        str.push_str("}");
        str
    }
    pub fn answer_list(&self) -> Vec<String> {
        let mut vec = Vec::with_capacity(1);
        // 从何处寻找answer
        let mut n = self.offset + 12; // offset + Header
        for _ in 0..self.qdcount() {
            loop {
                if self.value[n] == 0 {
                    n += 1 + 4; // 4 is QTYPE and QCLASS
                    break;
                }
                n += 1;
            }
        }

        fn b2a(b: &[u8]) -> String {
            match b.len() {
                4 => format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]),
                16 => {
                    let mut str = String::with_capacity(40);
                    for i in 0..b.len() {
                        str.push_str(&format!("{:02x}", b[i]));
                        if i % 2 == 1 {
                            str.push(':');
                        }
                    }
                    str.pop();
                    str.replace("0000", "").replace("::", ":")
                },
                _ => format!("{:02x?}", b),
            }
        }

        // 长度不定，可能是真正的数据，也有可能是指针（其值表示的是真正的数据在整个数据中的字节索引数），还有可能是二者的混合（以指针结尾）。
        // 若是真正的数据，会以0x00结尾；若是指针，指针占2个字节，第一个字节的高2位为11。
        for _ in 0..self.ancount() {
            if self.value[n] & 0b_1100_0000 == 0b_1100_0000 {
                let pointer =
                    (self.value[n] & 0b_0011_1111) as u16 * 256 + self.value[n + 1] as u16;
                let rdlength = self.value[n + pointer as usize - 2] as u16 * 256
                    + self.value[n + pointer as usize - 1] as u16; // RDLENGTH
                vec.push(b2a(
                    &self.value[n + pointer as usize..n + pointer as usize + rdlength as usize]
                ));
                n += (pointer + rdlength) as usize;
            }
        }
        vec
    }
    pub fn answer(&self) -> String {
        let mut str = String::with_capacity(1024);
        for answer in self.answer_list() {
            str.push_str(&format!("{}, ", answer));
        }
        str.pop();
        str.pop();
        str
    }
    pub fn info(&self) {
        // let target = self.question_list();
        // let target = target.get(0).unwrap();
        // if target.0 != "y2b.123345.xyz" { // test case
            // return;
        // }
        if self.qr() == "request" {
            println!(
                "{id} {opcode} {question}",
                opcode = self.opcode(),
                id = self.id(),
                question = self.question().magenta().bold(),
            );
        } else {
            println!(
                "{id} {opcode} {question} ({rcode})",
                rcode = self.rcode().green(),
                opcode = self.opcode(),
                id = self.id(),
                question = self.question().magenta().bold(),
            );
            println!(
                "                                      | {answer} |",
                answer = self.answer().red().bold(),
            );
        }
        // println!("{:02x?}", self.value); // 以十六进制而非十进制打印数组
        println!("{}", self.to_string().white().reversed());
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
        false
    } || DEBUG;
    let listener = tokio::net::UdpSocket::bind(listen).await;
    if let Err(e) = listener {
        println!("无法启用监听服务: {}", e);
        return Err(e);
    }
    let listener: UdpSocket = listener.ok().unwrap();
    let listener = std::sync::Arc::new(listener);
    println!("{}", "DNS代理服务已启动".red().bold());
    println!(
        "-----------------------------------------------------------------------------------"
    );

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
            println!("{}", "==>>  DNS查询  ==>>".cyan().bold());
            DNS::with(&query[..received + 2], 2).info();
        }

        let map1 = map.clone();
        let map2 = map.clone();
        // Find cache
        let cache = map1.lock().await;
        let cache = cache.get(&query[4..received + 2]);
        if let Some(cache) = cache {
            let mut buf = vec![0u8; 2 + cache.len()];
            buf[0] = query[2]; // ID
            buf[1] = query[3];
            &buf[2..2 + cache.len()].copy_from_slice(&cache[..]);
            if log {
                println!("{}", "                                      <<==  已缓存  <<==".blue().bold());
                print!("                                      ");
                DNS::with(&buf[..cache.len() + 2], 0).info();
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
                println!("{}", format!("无法连接服务器{}：{}", &server, e).red());
                return;
            }
            let mut tcp = tcp.ok().unwrap();

            // Faword query request
            let writed = tcp.write(&query[..received + 2]).await;
            if let Err(e) = writed {
                println!("{}", format!("意外的错误：{}", e).red());
                return;
            }

            let mut resp = [0u8; 2048];
            if let Ok(le) = tcp.read(&mut resp).await {
                let mut map = map2.lock().await;
                map.insert(Vec::from(&query[4..received + 2]), Vec::from(&resp[4..le]));
                listener.send_to(&resp[2..le], client).await.unwrap();
                if log {
                    println!("{}", "                                      <<==  DNS响应  <<==".green().bold());
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
