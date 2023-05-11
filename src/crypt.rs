#[cfg(debug)]
use ttycolor::*;
#[cfg(debug)]
use crate::dns::DNS;

pub fn encrypt(data: &mut [u8]) -> &mut [u8] {
    #[cfg(debug)] {
        println!("加密前：");
        println!("{}", DNS::with(data, 0).to_string().red().bold());
    }
    data.reverse();
    for i in 0..data.len() {
        let flag = data[i] & 0b_1100_0000;
        data[i] <<= 2;
        data[i] |= flag >> 6;
        data[i] ^= 0b_1011_0111;
    }
    #[cfg(debug)] {
        println!("加密后：");
        println!("{}", DNS::with(data, 0).to_string().red().bold());
    }
    data
}

pub fn decrypt(data: &mut [u8]) -> &mut [u8] {
    #[cfg(debug)] {
        println!("解密前：");
        println!("{}", DNS::with(data, 0).to_string().green().bold());
    }
    data.reverse();
    for i in 0..data.len() {
        data[i] ^= 0b_1011_0111;
        let flag = data[i] & 0b_0000_0011;
        data[i] >>= 2;
        data[i] |= flag << 6;
    }
    #[cfg(debug)] {
        println!("解密后：");
        println!("{}", DNS::with(data, 0).to_string().green().bold());
    }
    data
}