use std::collections::HashMap;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;

use ttycolor::*;

mod dns;
use dns::DNS;

const DEBUG: bool = false;

const LISTEN: &str = "0.0.0.0:53";
// const SERVER: &str = "1.1.1.1:53";
// const SERVER: &str = "114.114.114.114:53";
const SERVER: &str = "101.6.6.6:5353";

async fn client(crypt: bool) -> std::io::Result<()> {
    let listen= std::env::args().nth(if crypt { 1 } else { 2 }).unwrap_or(LISTEN.to_string());
    let server = std::env::args().nth(if crypt { 2 } else { 3 }).unwrap_or(SERVER.to_string());
    let log = std::env::args().nth(if crypt { 3 } else { 4 }).unwrap_or("on".to_string());
    let log = DEBUG || if let Some(_) = ["log_off", "off", "close"].iter().find(|&&it| (it == log.as_str())) { false } else { true };
    let listener = tokio::net::UdpSocket::bind(&listen).await;
    if let Err(e) = listener {
        println!("无法启用监听服务: {}", e);
        return Err(e);
    }
    let listener: UdpSocket = listener.ok().unwrap();
    let listener = std::sync::Arc::new(listener);
    if crypt {
        println!("{}: {} -> {}", "DNS代理服务client已启动".red().bold(), listen.cyan().bold(), server.green().bold());
    } else {
        println!("{}: {} -> {}", "DNS代理服务 (TCP转发) 已启动".red().bold(), listen.cyan().bold(), server.green().bold());
    }
    println!("{}", if log { "日志已开启" } else { "日志已关闭" }.red());
    println!(
        "-----------------------------------------------------------------------------------"
    );

    let map: HashMap<_, _> = HashMap::<Vec<u8>, Vec<u8>>::with_capacity(1024); // cache
    let map: tokio::sync::Mutex<_> = tokio::sync::Mutex::new(map); // Mutex<HashMap>
    let map: std::sync::Arc<_> = std::sync::Arc::new(map); // Arc<Mutex<HashMap>>

    let mut query = [0u8; 1024]; // DNS query request data
    loop {
        let server = server.clone();
        let listener = listener.clone();
        let recv_result = listener.recv_from(&mut query[2..]).await;
        if let Err(_) = recv_result {
            continue;
        }
        let (received, client) = recv_result.ok().unwrap();
        if log {
            println!("{}", "==>>  DNS查询  ==>>".cyan().bold());
            std::panic::catch_unwind(|| {
                DNS::with(&query[2..received + 2], 0).info();
            }).unwrap_or_default();
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
            buf[2..2 + cache.len()].copy_from_slice(&cache[..]);
            if log {
                println!("{}", "                                      <<==  已缓存  <<==".blue().bold());
                print!("                                      ");
                std::panic::catch_unwind(|| {
                    // DNS::with(&buf[..cache.len() + 2], 0).info();
                    DNS::with(&buf, 0).info();
                }).unwrap_or_default();
            }
            if let Err(e) = listener.send_to(&buf, client).await {
                eprintln!("Error: {}", e);
            }
            continue;
        }

        tokio::spawn(async move {
            // TCP data body length
            // 弃用
            query[0] = (received / 0xff) as u8;
            query[1] = (received % 0xff) as u8;

            // Connect server
            // let tcp = tokio::net::TcpStream::connect(&server).await;
            // let tcp = tokio::net::TcpStream::connect(&server).await;
            let udp = tokio::net::UdpSocket::bind("0.0.0.0:0").await;
            if let Err(e) = udp {
                println!("{}", format!("无法创建UDP：{}", e).red());
                return;
            }
            let udp = udp.ok().unwrap();
            udp.connect(server).await.unwrap_or_default();

            // Faword query request
            let mut backup = Vec::from(&query[2..received + 2]);
            let writed = if crypt {
                udp.send(encrypt(&mut backup)).await // 加密
            } else {
                udp.send(&mut backup).await // 加密
            };
            if let Err(e) = writed {
                println!("{}", format!("意外的错误：{}", e).red());
                return;
            }

            let mut resp = [0u8; 2048];
            if let Ok(le) = udp.recv(&mut resp).await {
                if crypt {
                    decrypt(&mut resp[..le]);
                }
                let mut map = map2.lock().await;
                map.insert(Vec::from(&query[4..received + 2]), Vec::from(&resp[2..le]));
                listener.send_to(&resp[..le], client).await.unwrap();
                if log {
                    println!("{}", "                                      <<==  DNS响应  <<==".green().bold());
                    print!("                                      ");
                    std::panic::catch_unwind(|| {
                        DNS::with(&resp[..le], 0).info();
                    }).unwrap_or_default();
                }
            }
        });
    }
}

mod crypt;
use crypt::*;

async fn server() -> std::io::Result<()> {
    let listen= std::env::args().nth(2).unwrap_or(LISTEN.to_string());
    let server = std::env::args().nth(3).unwrap_or(SERVER.to_string());
    let log = std::env::args().nth(4).unwrap_or("on".to_string());
    let log = DEBUG || if let Some(_) = ["log_off", "off", "close"].iter().find(|&&it| (it == log.as_str())) { false } else { true };
    let listener = tokio::net::UdpSocket::bind(&listen).await;
    if let Err(e) = listener {
        println!("无法启用监听服务: {}", e);
        return Err(e);
    }
    let listener: UdpSocket = listener.ok().unwrap();
    let listener = std::sync::Arc::new(listener);
    println!("{}: {} -> {}", "DNS代理服务server已启动".red().bold(), listen.cyan().bold(), server.green().bold());
    println!("{}", if log { "日志已开启" } else { "日志已关闭" }.red());
    println!(
        "-----------------------------------------------------------------------------------"
    );

    let map: HashMap<_, _> = HashMap::<Vec<u8>, Vec<u8>>::with_capacity(1024); // cache
    let map: tokio::sync::Mutex<_> = tokio::sync::Mutex::new(map); // Mutex<HashMap>
    let map: std::sync::Arc<_> = std::sync::Arc::new(map); // Arc<Mutex<HashMap>>

    let mut query = [0u8; 2048]; // DNS query request data
    loop {
        let server = server.clone();
        let listener = listener.clone();
        let recv_result = listener.recv_from(&mut query[2..]).await;
        if let Err(_) = recv_result {
            continue;
        }
        let (received, client) = recv_result.ok().unwrap();
        decrypt(&mut query[2..received + 2]); // 解密
        if log {
            println!("{}", "==>>  DNS查询  ==>>".cyan().bold());
            std::panic::catch_unwind(|| {
                DNS::with(&query[2..received + 2], 0).info();
            }).unwrap_or_default();
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
            buf[2..2 + cache.len()].copy_from_slice(&cache[..]);
            if log {
                println!("{}", "                                      <<==  已缓存  <<==".blue().bold());
                print!("                                      ");
                std::panic::catch_unwind(|| {
                    DNS::with(&buf[..cache.len() + 2], 0).info();
                }).unwrap_or_default();
            }
            if let Err(e) = listener.send_to(encrypt(Vec::from(&buf[..]).as_mut_slice()), client).await {  // 加密
                eprintln!("Error: {}", e);
            }
            continue;
        }

        tokio::spawn(async move {
            // TCP data body length
            query[0] = (received / 0xff) as u8;
            query[1] = (received % 0xff) as u8;

            #[cfg(tcp)] {
                // Connect server
                let tcp = tokio::net::TcpStream::connect(&server).await;
                if let Err(e) = tcp {
                    println!("{}", format!("无法连接服务器{}：{}", server, e).red());
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
                    // listener.send_to(&resp[2..le], client).await.unwrap();
                    listener.send_to(encrypt(Vec::from(&resp[2..le]).as_mut_slice()), client).await.unwrap(); // 加密
                    if log {
                        println!("{}", "                                      <<==  DNS响应  <<==".green().bold());
                        print!("                                      ");
                        std::panic::catch_unwind(|| {
                            DNS::with(&resp[2..le], 0).info();
                        }).unwrap_or_default();
                    }
                }
            }
            
            #[cfg(not(tcp))] {
                // Connect server
                let udp = tokio::net::UdpSocket::bind("0.0.0.0:0").await;
                if let Err(e) = udp {
                    println!("{}", format!("无法创建UDP：{}", e).red());
                    return;
                }
                let udp = udp.ok().unwrap();
                udp.connect(&server).await.unwrap();

                // Faword query request
                let writed = udp.send(&query[2..received + 2]).await;
                if let Err(e) = writed {
                    println!("{}", format!("意外的错误：{}", e).red());
                    return;
                }

                udp.connect(&server).await.unwrap();
                let mut resp = [0u8; 2048];
                if let Ok(le) = udp.recv(&mut resp).await {
                    let mut map = map2.lock().await;
                    map.insert(Vec::from(&query[4..received + 2]), Vec::from(&resp[2..le]));
                    // listener.send_to(&resp[2..le], client).await.unwrap();
                    listener.send_to(encrypt(Vec::from(&resp[..le]).as_mut_slice()), client).await.unwrap(); // 加密
                    if log {
                        println!("{}", "                                      <<==  DNS响应  <<==".green().bold());
                        print!("                                      ");
                        std::panic::catch_unwind(|| {
                            DNS::with(&resp[..le], 0).info();
                        }).unwrap_or_default();
                    }
                }
            }
        });
    }
}

async fn client_tcp() -> std::io::Result<()> {
    let listen= std::env::args().nth(2).unwrap_or(LISTEN.to_string());
    let server = std::env::args().nth(3).unwrap_or(SERVER.to_string());
    let log = std::env::args().nth(4).unwrap_or("on".to_string());
    let log = DEBUG || if let Some(_) = ["log_off", "off", "close"].iter().find(|&&it| (it == log.as_str())) { false } else { true };
    let listener = tokio::net::UdpSocket::bind(&listen).await;
    if let Err(e) = listener {
        println!("无法启用监听服务: {}", e);
        return Err(e);
    }
    let listener: UdpSocket = listener.ok().unwrap();
    let listener = std::sync::Arc::new(listener);
    println!("{}: {} -> {}", "DNS代理服务 (TCP转发) 已启动".red().bold(), listen.cyan().bold(), server.green().bold());
    println!("{}", if log { "日志已开启" } else { "日志已关闭" }.red());
    println!(
        "-----------------------------------------------------------------------------------"
    );

    let map: HashMap<_, _> = HashMap::<Vec<u8>, Vec<u8>>::with_capacity(1024); // cache
    let map: tokio::sync::Mutex<_> = tokio::sync::Mutex::new(map); // Mutex<HashMap>
    let map: std::sync::Arc<_> = std::sync::Arc::new(map); // Arc<Mutex<HashMap>>

    let mut query = [0u8; 1024]; // DNS query request data
    loop {
        let server = server.clone();
        let listener = listener.clone();
        let recv_result = listener.recv_from(&mut query[2..]).await;
        if let Err(_) = recv_result {
            continue;
        }
        let (received, client) = recv_result.ok().unwrap();
        if log {
            println!("{}", "==>>  DNS查询  ==>>".cyan().bold());
            std::panic::catch_unwind(|| {
                DNS::with(&query[..received + 2], 2).info();
            }).unwrap_or_default();
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
            buf[2..2 + cache.len()].copy_from_slice(&cache[..]);
            if log {
                println!("{}", "                                      <<==  已缓存  <<==".blue().bold());
                print!("                                      ");
                std::panic::catch_unwind(|| {
                    DNS::with(&buf[..cache.len() + 2], 0).info();
                }).unwrap_or_default();
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
            let tcp = tokio::net::TcpStream::connect(&server).await;
            if let Err(e) = tcp {
                println!("{}", format!("无法连接服务器{}：{}", server, e).red());
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
                    std::panic::catch_unwind(|| {
                        DNS::with(&resp[..le], 2).info();
                    }).unwrap_or_default();
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    match std::env::args().nth(1).unwrap_or_default().as_str() {
        "server" => {
            server().await
        }
        "tcp" => {
            client_tcp().await
        }
        "udp" => {
            client(false).await
        }
        _ => client(true).await
    }
}
