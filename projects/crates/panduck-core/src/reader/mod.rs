#![doc = include_str!("readme.md")]

pub use self::token::Token;
use crate::PanduckError;
use byteorder::{ByteOrder, ReadBytesExt};
use std::{
    io::{Read, Seek, SeekFrom},
    marker::PhantomData,
};

mod token;

/// 二进制读取器，用于从实现了 ReadBytesExt trait 的类型中读取数据
///
/// 这是一个泛型结构体，可以包装任何实现了 ReadBytesExt trait 的类型，
/// 提供二进制数据的读取功能。
#[derive(Debug)]
pub struct BinaryReader<R, E> {
    reader: R,
    position: u64,
    endian: PhantomData<E>,
}

impl<R: Read, E> Read for BinaryReader<R, E> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buffer)
    }
}

impl<R, E> BinaryReader<R, E> {
    /// 创建新的二进制读取器
    ///
    /// # Arguments
    /// * `lexer` - 要读取的数据
    ///
    /// # Returns
    /// 返回新的 BinaryReader 实例
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            position: 0,
            endian: Default::default(),
        }
    }

    /// 获取当前读取位置
    ///
    /// # Returns
    /// 返回当前的字节偏移位置
    pub fn get_position(&self) -> u64 {
        self.position
    }

    /// 设置读取位置
    ///
    /// 注意：如果底层reader不支持Seek操作，此函数将返回错误
    ///
    /// # Arguments
    /// * `pos` - 新的读取位置
    ///
    /// # Returns
    /// 返回操作结果
    pub fn set_position(&mut self, position: u64) -> Result<u64, PanduckError>
    where
        R: Seek,
    {
        self.reader.seek(SeekFrom::Start(position))?;
        self.position = position;
        Ok(position)
    }
    pub fn finish(self) -> R {
        self.reader
    }
}

impl<R: ReadBytesExt, E: ByteOrder> BinaryReader<R, E> {
    /// 读取 u8
    ///
    /// # Returns
    /// 返回读取的 u8 值或 IO 错误
    pub fn read_u8(&mut self) -> std::io::Result<u8> {
        let value = self.reader.read_u8()?;
        self.position += 1;
        Ok(value)
    }

    /// 读取 u16
    ///
    /// # Returns
    /// 返回读取的 u16 值或 IO 错误
    pub fn read_u16(&mut self) -> std::io::Result<u16> {
        let value = self.reader.read_u16::<E>()?;
        self.position += 2;
        Ok(value)
    }

    /// 读取 i16
    ///
    /// # Returns
    /// 返回读取的 i16 值或 IO 错误
    pub fn read_i16(&mut self) -> std::io::Result<i16> {
        let value = self.reader.read_i16::<E>()?;
        self.position += 2;
        Ok(value)
    }

    /// 读取 u32
    ///
    /// # Returns
    /// 返回读取的 u32 值或 IO 错误
    pub fn read_u32(&mut self) -> std::io::Result<u32> {
        let value = self.reader.read_u32::<E>()?;
        self.position += 4;
        Ok(value)
    }

    /// 读取 u64
    ///
    /// # Returns
    /// 返回读取的 u64 值或 IO 错误
    pub fn read_u64(&mut self) -> std::io::Result<u64> {
        let value = self.reader.read_u64::<E>()?;
        self.position += 8;
        Ok(value)
    }

    /// 读取指定长度的字节数组
    ///
    /// # Arguments
    /// * `len` - 要读取的字节数
    ///
    /// # Returns
    /// 返回读取的字节数组或 IO 错误
    pub fn read_bytes(&mut self, len: usize) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.reader.read_exact(&mut buf)?;
        self.position += len as u64;
        Ok(buf)
    }

    /// 读取固定长度的字节数组
    ///
    /// # Returns
    /// 返回读取的字节数组或 IO 错误
    pub fn read_array<const N: usize>(&mut self) -> std::io::Result<[u8; N]> {
        let mut buf = [0u8; N];
        self.reader.read_exact(&mut buf)?;
        self.position += N as u64;
        Ok(buf)
    }

    /// 跳过指定数量的字节
    ///
    /// 注意：如果底层reader不支持Seek操作，此函数将返回错误
    ///
    /// # Arguments
    /// * `count` - 要跳过的字节数
    ///
    /// # Returns
    /// 返回操作结果
    pub fn skip(&mut self, count: u64) -> std::io::Result<u64>
    where
        R: Seek,
    {
        let new_pos = self.reader.seek(SeekFrom::Current(count as i64))?;
        self.position = new_pos;
        Ok(new_pos)
    }
}

impl<R: Seek, E: ByteOrder> BinaryReader<R, E> {
    pub fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(pos)
    }
}
