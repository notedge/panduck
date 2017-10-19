#![doc = include_str!("readme.md")]

use byteorder::{ByteOrder, WriteBytesExt};
use std::{
    io::{Seek, SeekFrom, Write},
    marker::PhantomData,
};

/// 二进制写入器，用于从实现了 WriteBytesExt trait 的类型中写入数据
///
/// 这是一个泛型结构体，可以包装任何实现了 WriteBytesExt trait 的类型，
/// 提供二进制数据的写入功能。
#[derive(Copy, Clone, Debug)]
pub struct BinaryWriter<W, E> {
    writer: W,
    endian: PhantomData<E>,
}

impl<W: Write, E> Write for BinaryWriter<W, E> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Seek, E> Seek for BinaryWriter<W, E> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.writer.seek(pos)
    }
}

impl<W, E> BinaryWriter<W, E> {
    /// 创建一个新的二进制写入器
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            endian: PhantomData,
        }
    }

    /// 获取内部写入器
    pub fn finish(self) -> W {
        self.writer
    }

    /// Writes one unsigned 8-bit integer.
    pub fn write_u8(&mut self, value: u8) -> std::io::Result<()>
    where
        W: Write,
    {
        self.writer.write_u8(value)
    }

    /// Writes one unsigned 16-bit integer using endianness `E`.
    pub fn write_u16(&mut self, value: u16) -> std::io::Result<()>
    where
        W: Write,
        E: ByteOrder,
    {
        self.writer.write_u16::<E>(value)
    }

    /// Writes one unsigned 32-bit integer using endianness `E`.
    pub fn write_u32(&mut self, value: u32) -> std::io::Result<()>
    where
        W: Write,
        E: ByteOrder,
    {
        self.writer.write_u32::<E>(value)
    }

    /// Writes one signed 32-bit integer using endianness `E`.
    pub fn write_i32(&mut self, value: i32) -> std::io::Result<()>
    where
        W: Write,
        E: ByteOrder,
    {
        self.writer.write_i32::<E>(value)
    }

    /// Writes one unsigned 64-bit integer using endianness `E`.
    pub fn write_u64(&mut self, value: u64) -> std::io::Result<()>
    where
        W: Write,
        E: ByteOrder,
    {
        self.writer.write_u64::<E>(value)
    }

    /// Writes a raw byte slice without endian conversion.
    pub fn write_bytes(&mut self, bytes: &[u8]) -> std::io::Result<()>
    where
        W: Write,
    {
        self.writer.write_all(bytes)
    }
}
