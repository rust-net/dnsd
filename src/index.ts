import * as net from 'net';
import * as udp from 'dgram';
import printMsg from './printMsg';
import tcpServer from './tcpServer';

const config: ServerConfig = {
    host: 'localhost',
    port: 53,
    server: '1.1.1.1',
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

            // add length
            var tcpQuery = Buffer.alloc(2 + msg.length);
            tcpQuery[0] = msg.length / 256;
            tcpQuery[1] = msg.length % 256;
            for (var i = 2; i < tcpQuery.length; i++) {
                tcpQuery[i] = msg[i - 2];
            }
            printMsg(tcpQuery, 'forward');

            tcp.write(tcpQuery);
            tcp.once('data', (resp) => {
                // delete length
                var tcpResp = Buffer.alloc(msg.length);
                for (var i = 0; i < tcpResp.length; i++) {
                    tcpResp[i] = resp[i + 2];
                }
                printMsg(tcpResp, 'resp');
                server.send(tcpResp, rinfo.port, rinfo.address);
                tcp.destroy();
            });
        });

        tcp.on('error', (err) => {
            console.error(err);
        });
    });
}

(() => {
    tcpServer(config);
    udpServer(config);

    process.on('uncaughtException', () => {
        console.log('ERROR !!!!!!!!');
    });
})();
