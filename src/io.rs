use crate::error::{Error, Result};

pub(crate) fn put_varint(buf: &mut Vec<u8>, mut value: u32) {
    loop {
        let byte = (value & 0x7F) as u8;

        value >>= 7;

        if value == 0 {
            buf.push(byte);

            return;
        }

        buf.push(byte | 0x80);
    }
}

pub(crate) fn put_len(buf: &mut Vec<u8>, len: usize) -> Result<()> {
    put_varint(buf, u32::try_from(len).map_err(|_| Error::TooLarge)?);

    Ok(())
}

pub(crate) fn put_string(buf: &mut Vec<u8>, s: &str) -> Result<()> {
    put_len(buf, s.len())?;
    buf.extend_from_slice(s.as_bytes());

    Ok(())
}

pub(crate) fn vec3_bytes(v: [f32; 3]) -> [u8; 12] {
    let mut out = [0u8; 12];

    for (i, value) in v.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }

    out
}

pub(crate) struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn rest(&self) -> &'a [u8] {
        self.buf
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.buf.len() {
            return Err(Error::UnexpectedEof);
        }

        let (head, tail) = self.buf.split_at(n);

        self.buf = tail;

        Ok(head)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16> {
        let b = self.bytes(2)?;

        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn f32(&mut self) -> Result<f32> {
        let b = self.bytes(4)?;

        Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn vec3(&mut self) -> Result<[f32; 3]> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }

    pub fn varint(&mut self) -> Result<u32> {
        let mut result: u32 = 0;

        for i in 0..5 {
            let byte = self.u8()?;
            let bits = u32::from(byte & 0x7F);

            if i == 4 && bits > 0x0F {
                return Err(Error::VarintOverflow);
            }

            result |= bits << (7 * i);

            if byte & 0x80 == 0 {
                return Ok(result);
            }
        }

        Err(Error::VarintOverflow)
    }
}
