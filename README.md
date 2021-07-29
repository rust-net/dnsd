# dnsd

DNS 污染是一种让一般用户由于得到虚假目标主机IP而不能与其通信的方法，是一种 DNS 缓存投毒攻击（DNS cache poisoning）。其工作方式是：由于通常的 DNS 查询没有任何认证机制，而且 DNS 查询通常基于无连接不可靠的 UDP 协议，因此 DNS 的查询非常容易被篡改，通过对 UDP 数据传输进行侦听，筛选 DNS 查询，一经发现黑名单上的 DNS 请求则立即伪装成目标域名的解析服务器（NS，Name Server）给查询者返回虚假结果，同时也会篡改服务器发送的 DNS 响应。

dnsd 使用 TCP 传输协议代理 DNS 请求，可提高 DNS 污染的成本，获得相对真实的 DNS 响应。但由于明文传输，TCP 请求仍有可能被重置、中断。如果您没有 TCP 全局代理的解决方案，那么可以尝试在相对安全的网络服务器上搭建一个加密的 DNS 服务，尝试使用 [client-server](https://github.com/develon2015/dnsd/tree/client-server) 架构。


## Usage
```
./dnsd 127.0.0.1:53 1.1.1.1:53 log_on
```

![./res/dsnds.png](https://raw.githubusercontent.com/develon2015/dnsd/rust/res/dnsd_win.png)
