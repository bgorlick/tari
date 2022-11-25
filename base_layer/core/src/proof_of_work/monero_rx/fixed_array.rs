//  Copyright 2021, The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::{convert::TryFrom, io, io::Write, ops::Deref};

use borsh::{BorshDeserialize, BorshSerialize};
use tari_utilities::{ByteArray, ByteArrayError};

const MAX_ARR_SIZE: usize = 63;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedByteArray {
    elems: [u8; MAX_ARR_SIZE],
    len: u8,
}

impl BorshSerialize for FixedByteArray {
    fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.len.serialize(writer)?;
        let data = self.as_slice();
        for item in data.iter().take(self.len as usize) {
            item.serialize(writer)?;
        }
        Ok(())
    }
}

impl BorshDeserialize for FixedByteArray {
    fn deserialize(buf: &mut &[u8]) -> io::Result<Self> {
        let len = u8::deserialize(buf)? as usize;
        if len > MAX_ARR_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("length exceeded maximum of 63-bytes for FixedByteArray: {}", len),
            ));
        }
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push(u8::deserialize(buf)?);
        }
        // This unwrap should never fail, the len is checked above.
        Ok(Self::from_bytes(bytes.as_bytes()).unwrap())
    }
}

impl FixedByteArray {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self[..self.len()]
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == MAX_ARR_SIZE
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Deref for FixedByteArray {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.elems[..self.len as usize]
    }
}

#[allow(clippy::derivable_impls)]
impl Default for FixedByteArray {
    fn default() -> Self {
        Self {
            elems: [0u8; MAX_ARR_SIZE],
            len: 0,
        }
    }
}

impl ByteArray for FixedByteArray {
    fn from_bytes(bytes: &[u8]) -> Result<Self, ByteArrayError> {
        if bytes.len() > MAX_ARR_SIZE {
            return Err(ByteArrayError::IncorrectLength);
        }

        let len = u8::try_from(bytes.len()).map_err(|_| ByteArrayError::IncorrectLength)?;

        let mut elems = [0u8; MAX_ARR_SIZE];
        elems[..len as usize].copy_from_slice(&bytes[..len as usize]);
        Ok(Self { elems, len })
    }

    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn assert_size() {
        assert_eq!(std::mem::size_of::<FixedByteArray>(), MAX_ARR_SIZE + 1);
    }

    #[test]
    fn from_bytes() {
        let arr = FixedByteArray::from_bytes(&[1u8][..]).unwrap();
        assert_eq!(arr.len(), 1);
        assert!(arr.iter().all(|b| *b == 1));
        // Iterates only up to len
        let mut used = false;
        for _ in arr.iter() {
            assert!(!used);
            used = true;
        }
        assert!(used);

        let arr = FixedByteArray::from_bytes(&[1u8; 63][..]).unwrap();
        assert_eq!(arr.len(), 63);
        assert!(arr.iter().all(|b| *b == 1));

        FixedByteArray::from_bytes(&[1u8; 64][..]).unwrap_err();
    }

    // #[test]
    // fn length_check() {
    //     let mut buf = [0u8; MAX_ARR_SIZE + 1];
    //     buf[0] = 63;
    //     let arr = FixedByteArray::consensus_decode(&mut io::Cursor::new(buf)).unwrap();
    //     assert_eq!(arr.len(), MAX_ARR_SIZE);

    //     buf[0] = 64;
    //     let _err = FixedByteArray::consensus_decode(&mut io::Cursor::new(buf)).unwrap_err();
    // }

    #[test]
    fn capacity_overflow_does_not_panic() {
        let data = &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f];
        let _result = FixedByteArray::deserialize(&mut data.as_slice()).unwrap_err();
    }

    #[test]
    fn length_check() {
        let mut buf = [MAX_ARR_SIZE as u8; MAX_ARR_SIZE + 1];
        let fixed_byte_array = FixedByteArray::deserialize(&mut buf.as_slice()).unwrap();
        assert_eq!(fixed_byte_array.len(), MAX_ARR_SIZE);
        buf[0] += 1;
        FixedByteArray::deserialize(&mut buf.as_slice()).unwrap_err();
    }

    #[test]
    fn test_borsh_de_serialization() {
        let fixed_byte_array = FixedByteArray::from_bytes(&[5, 6, 7]).unwrap();
        let mut buf = Vec::new();
        fixed_byte_array.serialize(&mut buf).unwrap();
        buf.extend_from_slice(&[1, 2, 3]);
        let buf = &mut buf.as_slice();
        assert_eq!(fixed_byte_array, FixedByteArray::deserialize(buf).unwrap());
        assert_eq!(buf, &[1, 2, 3]);
    }
}
