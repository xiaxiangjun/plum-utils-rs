use std::fmt::Error;
use std::fmt::LowerHex;
use std::mem::size_of;
use std::ops::{BitAnd, BitOrAssign, Shl};

pub struct BigEndian<'a>(&'a [u8]);

impl<'a> BigEndian<'a> {
    pub fn get<T>(self) -> Result<T, Error>
    where
        T: Sized
            + BitOrAssign
            + Shl<T>
            + Default
            + Shl<Output = T>
            + BitAnd<Output = T>
            + From<u8>
            + Copy
            + LowerHex,
    {
        let mut r: T = Default::default();
        let s = size_of::<T>();

        // 判断大小是否足够
        if self.0.len() < s {
            return Err(std::fmt::Error);
        }

        for i in 0..s {
            println!(">>> {}-1: {:x}", i, r);
            if i > 0 {
                r |= (r << 8u8.into()) & 0xff.into();
            }

            println!(">>> {}-2: {:x}", i, r);
            let b: T = self.0[i].into();
            r |= b & 0xff.into();
            println!(">>> {}-3: {:x}", i, r);
        }

        Ok(r)
    }
}

#[test]
fn test_big_endian_get() -> Result<(), Error> {
    let data = [0x11u8, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18];

    println!("{:x}", BigEndian(&data).get::<u16>()?);

    Ok(())
}
