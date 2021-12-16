use std::io::{Error, Read};

pub struct LoopReader {
    data: Vec<u8>,
    pos: usize,
}

impl LoopReader {
    // 创建一个新的对像
    pub fn new<T: Read>(mut r: T) -> Result<Self, Error> {
        // 读取全部文件
        let mut data: Vec<u8> = Vec::new();
        r.read_to_end(&mut data)?;

        Ok(LoopReader { data, pos: 0 })
    }
}

impl Read for LoopReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        // 将游标移动最开始
        if self.pos >= self.data.len() {
            self.pos = 0;
        }

        // 读取要copy的数据大小
        let size = if self.pos + buf.len() > self.data.len() {
            self.data.len() - self.pos
        } else {
            buf.len()
        };

        // 读取数据
        if buf.len() > size  {
            let (left, _) = buf.split_at_mut(size);
            left.copy_from_slice(&self.data[self.pos..self.pos + size]);
        } else {
            buf.copy_from_slice(&self.data[self.pos..self.pos + size]);
        }

        // 指针要下移
        self.pos += size;

        Ok(size)
    }
}
