import * as net from 'net';
import printMsg from './printMsg';

export default function tcpServer(config: ServerConfig) {
    var tcp = net.createServer();
    tcp.listen(config.port, config.host, () => {
        tcp.on('connection', (conn) => {
            conn.on('data', (msg) => {
                printMsg(msg, 'A Query');

                var dnsServer = config.server || '1.1.1.1';
                var forward = net.createConnection({ host: dnsServer, port: 53 }, () => {
                    console.log('server connected...');
                    forward.write(msg);
                    forward.once('data', (resp) => {
                        printMsg(resp, 'resp');
                        conn.write(resp);
                    });
                });

                forward.on('error', (err) => {
                    console.error(err);
                });
            });
        });
    });

    tcp.on('error', (err) => {
        console.error('err');
    });
}
