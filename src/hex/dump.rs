use std::fmt::{Error, Formatter, LowerHex};

const HEX_DATA: [&str; 18] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F", "\x20", "\r\n",
];

// 定义dump结构体
pub struct Dump<'a> {
    data: &'a [u8],
    max: usize,  // default data.len()
    wrap: usize, // default not line wrap
}

impl<'a> Dump<'a> {
    // 创建一个切片的dump
    pub fn slice(a: &'a [u8]) -> Self {
        Dump {
            data: a,
            max: a.len(),
            wrap: a.len(),
        }
    }

    pub fn with_max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    pub fn with_line_wrap(mut self, wrap: usize) -> Self {
        self.wrap = wrap;
        self
    }
}

// 支持 println!("{:x}", Dump(&[u8])) 操作
impl<'a> LowerHex for Dump<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        for (index, n) in self.data.iter().enumerate() {
            // 判断是否超过大小
            if index >= self.max {
                break;
            }

            // 判断是否要换行
            if index > 0 && self.wrap > 0 && 0 == index % self.wrap {
                f.write_str(HEX_DATA[17])?;
            }

            f.write_str(HEX_DATA[((*n >> 4) & 0xf) as usize])?;
            f.write_str(HEX_DATA[(*n & 0xf) as usize])?;
            f.write_str(HEX_DATA[16])?;
        }

        Ok(())
    }
}

#[test]
fn test_dump() {
    let data = [0x11u8, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17];

    println!("{:x}", Dump::slice(&data).with_max(5).with_line_wrap(3));
}
