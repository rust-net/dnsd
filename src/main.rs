use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;

const LISTEN: &str = "0.0.0.0:53";
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

            if let Ok(le) = tcp.read(&mut query).await {
                listener.send_to(&query[2..le], client).await.unwrap();
                if log {
                    println!("resp: {:?}", &query[..le]);
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    udp_serv().await
}
