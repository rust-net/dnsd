interface ServerConfig {
    host: string;
    port: number;
    server: string;
}

/**
 * Header部分是一定有的，长度固定为12个字节；其余4部分可能有也可能没有，并且长度也不一定
 */
interface DNSHeader {
    /** 16 bit */
    ID: number; // 2x8

    /** 1 bit, 指示该消息是请求还是响应。0表示请求；1表示响应。 */
    QR: 0 | 1;

    /** 4 bit, 指示请求的类型，由请求发起者设定，响应消息中复用该值。0表示标准查询；1表示反转查询；2表示服务器状态查询。3~15目前保留，以备将来使用。 */
    OPCODE: 0 | 1 | 2;

    /** 1 bit, 表示响应的服务器是否是权威DNS服务器。只在响应消息中有效。 */
    AA: 0 | 1;

    /** 1 bit, 指示消息是否因为传输大小限制而被截断。 */
    TC: 0 | 1;

    /** 1 bit, 该值在请求消息中被设置，响应消息复用该值。如果被设置，表示希望服务器递归查询。但服务器不一定支持递归查询。 */
    RD: 0 | 1; // 3x8

    /** RA（Recursion Available，递归可用性）：1 bit, 该值在响应消息中被设置或被清除，以表明服务器是否支持递归查询。 */
    RA: 0 | 1;

    /** Z：占3位。保留备用。 */
    Z: 0;

    /**
     * RCODE（Response code）：4 bit, 该值在响应消息中被设置。取值及含义如下：
            0：No error condition，没有错误条件；
            1：Format error，请求格式有误，服务器无法解析请求；
            2：Server failure，服务器出错。
            3：Name Error，只在权威DNS服务器的响应中有意义，表示请求中的域名不存在。
            4：Not Implemented，服务器不支持该请求类型。
            5：Refused，服务器拒绝执行请求操作。
            6~15：保留备用。
     */
    RCODE: 0 | 1 | 2 | 3 | 4 | 5; // 4x8

    /** QDCOUNT：16 bit, 指明Question部分的包含的实体数量。 */
    QDCOUNT: number; // 6x8
    /** ANCOUNT：16 bit, 指明Answer部分的包含的RR（Resource Record）数量。 */
    ANCOUNT: number; // 8x8
    /** NSCOUNT：16 bit, 指明Authority部分的包含的RR（Resource Record）数量。 */
    NSCOUNT: number; // 10x8
    /** ARCOUNT：16 bit, 指明Additional部分的包含的RR（Resource Record）数量。 */
    ARCOUNT: number; // 12x8
}
