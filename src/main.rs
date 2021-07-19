use std::collections::HashMap;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;

const LISTEN: &str = "127.0.0.35:53";
const SERVER: &str = "1.1.1.1:53";

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
            println!("DNS查询...");
            println!("data: {:?}", &query[..received + 2]);
        }

        let map1= map.clone();
        let map2= map.clone();
        // Find cache
        let cache = map1.lock().await;
        let cache = cache.get(&query[4..received + 2]);
        if let Some(cache) = cache {
            if log {
                println!("该查询被缓存");
            }
            let mut buf = vec![0u8; 2 + cache.len()];
            buf[0] = query[2];
            buf[1] = query[3];
            &buf[2..2+cache.len()].copy_from_slice(&cache[..]);
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
                    println!("resp: {:?}", &resp[..le]);
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    udp_serv().await
}
