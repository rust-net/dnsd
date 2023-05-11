# dnsd

DNS 污染是一种让一般用户由于得到虚假目标主机IP而不能与其通信的方法，是一种 DNS 缓存投毒攻击（DNS cache poisoning）。其工作方式是：由于通常的 DNS 查询没有任何认证机制，而且 DNS 查询通常基于无连接不可靠的 UDP 协议，因此 DNS 的查询非常容易被篡改，通过对 UDP 数据传输进行侦听，筛选 DNS 查询，一经发现黑名单上的 DNS 请求则立即伪装成目标域名的解析服务器（NS，Name Server）给查询者返回虚假结果，同时也会篡改服务器发送的 DNS 响应。

dnsd: client-server 通过加密数据传输来代理 DNS 请求。


## Usage

server:
```
./dnsd server 0.0.0.0:1234 1.1.1.1:53 log_off
```

client:
```
./dnsd 127.0.0.1:53 your_server:1234 log_off
```

客户端兼容不加密的 tcp 模式：
```
./dnsd tcp 127.0.0.1:53 1.1.1.1:53 log_off
```

![./res/dsnds.png](https://raw.githubusercontent.com/develon2015/dnsd/rust/res/dnsd_win.png)
