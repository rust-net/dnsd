export default function printMsg(msg: Buffer, hint: string) {
    console.log(`===============${hint}:==================`);
    console.log(msg.buffer);

    var le = msg.byteLength;
    var str = '';
    for (let i = 0; i < le; i++) {
        str += String.fromCharCode(msg[i]);
    }
    console.log(`try read:`, str);
}
