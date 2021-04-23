use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;

const LISTEN: &str = "0.0.0.0:53";
const SERVER: &str = "1.1.1.1:53";

async fn udp_serv() -> std::io::Result<()> {
    let listener = tokio::net::UdpSocket::bind(LISTEN).await;
    if let Err(e) = listener {
        println!("无法启用监听服务: {}", e);
        return Err(e);
    }
    let listener: UdpSocket = listener.ok().unwrap();
    let listener = std::sync::Arc::new(listener);

    loop {
        let listener = listener.clone();
        let mut query = [0u8; 10240*2]; // DNS query request data
        let recv_result = listener.recv_from(&mut query[2..]).await;
        if let Err(_) = recv_result {
            continue;
        }
        let (received, client) = recv_result.ok().unwrap();
        println!("DNS查询...");
        println!("data: {:?}", &query[..received + 2]);

        tokio::spawn(async move {
            // TCP data body length
            query[0] = (received / 0xff) as u8;
            query[1] = (received % 0xff) as u8;

            // Connect server
            let tcp = tokio::net::TcpStream::connect(SERVER).await;
            if let Err(e) = tcp {
                println!("无法连接服务器{}：{}", SERVER, e);
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
                listener
                    .send_to(&query[2..le], client)
                    .await
                    .unwrap();
                println!("resp: {:?}", &query[..le]);
            }
        });
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    udp_serv().await
}
