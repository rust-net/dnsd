import * as net from 'net';
import * as udp from 'dgram';
import printMsg from './printMsg';
import tcpServer from './tcpServer';

const config: ServerConfig = {
    host: 'localhost',
    port: 53,
    server: '8.8.8.8',
};

function udpServer(config: ServerConfig) {
    var server = udp.createSocket('udp4');
    server.bind({ port: config.port, address: config.host, }, () => {
        task(server, config);
    });
    server.on('error', (err) => {
        console.error(err);
    });
}

function task(server: udp.Socket, config: ServerConfig) {
    server.on('message', (msg, rinfo) => {
        console.log(rinfo);
        printMsg(msg, 'conn');

        // forward to DNS server
        var dnsServer = config.server || '1.1.1.1';
        var tcp = net.createConnection({ host: dnsServer, port: 53 }, () => {
            console.log('server connected...');

            // add "0x00 0x33"
            var tcpQuery = Buffer.alloc(2 + msg.length);
            tcpQuery[0] = 0x00;
            tcpQuery[1] = 0x33;
            for (var i = 2; i < tcpQuery.length; i++) {
                tcpQuery[i] = msg[i - 2];
            }
            printMsg(tcpQuery, 'forward');

            tcp.write(tcpQuery);
            tcp.once('data', (resp) => {
                // delete "0x00 0x33"
                var tcpResp = Buffer.alloc(msg.length);
                for (var i = 0; i < tcpResp.length; i++) {
                    tcpResp[i] = resp[i + 2];
                }
                printMsg(tcpResp, 'resp');
                server.send(tcpResp, rinfo.port, rinfo.address);
            });
        });
    });
}

(() => {
    tcpServer(config);
    udpServer(config);
})();
