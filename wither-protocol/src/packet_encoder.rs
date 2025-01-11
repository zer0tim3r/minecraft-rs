use aes::cipher::{generic_array::GenericArray, BlockEncryptMut, BlockSizeUser, KeyIvInit};
use bytes::{BufMut, BytesMut};
use thiserror::Error;

use libdeflater::{CompressionLvl, Compressor};

use crate::{
    codec::Codec, CompressionLevel, CompressionThreshold, Packet, VarInt, MAX_PACKET_SIZE
};

type Cipher = cfb8::Encryptor<aes::Aes128>;

/// Encoder: Server -> Client
/// Supports ZLib endecoding/compression
/// Supports Aes128 Encryption
#[derive(Default)]
pub struct PacketEncoder {
    buf: BytesMut,
    compress_buf: Vec<u8>,
    cipher: Option<Cipher>,
    // compression and compression threshold
    compression: Option<(Compressor, CompressionThreshold)>,
}

impl PacketEncoder {
    /// If compression is enabled and the packet size exceeds the threshold, the packet is compressed.
    /// The packet is prefixed with its length and, if compressed, the uncompressed data length.
    /// The packet format is as follows:
    ///
    /// **Uncompressed:**
    /// |-----------------------|
    /// | Packet Length (VarInt)|
    /// |-----------------------|
    /// | Packet ID (VarInt)    |
    /// |-----------------------|
    /// | Data (Byte Array)     |
    /// |-----------------------|
    ///
    /// **Compressed:**
    /// |------------------------|
    /// | Packet Length (VarInt) |
    /// |------------------------|
    /// | Data Length (VarInt)   |
    /// |------------------------|
    /// | Packet ID (VarInt)     |
    /// |------------------------|
    /// | Data (Byte Array)      |
    /// |------------------------|
    ///
    /// -   `Packet Length`: The total length of the packet *excluding* the `Packet Length` field itself.
    /// -   `Data Length`: (Only present in compressed packets) The length of the uncompressed `Packet ID` and `Data`.
    /// -   `Packet ID`: The ID of the packet.
    /// -   `Data`: The packet's data.
    pub fn append_packet<P: Packet>(&mut self, packet: &P) -> Result<(), PacketEncodeError> {
        let start_len = self.buf.len();
        // Write the Packet ID first
        VarInt(P::PACKET_ID).encode(&mut self.buf);
        // Now write the packet into an empty buffer
        packet.write(&mut self.buf);
        let data_len = self.buf.len() - start_len;

        if let Some((compressor, compression_threshold)) = &mut self.compression {
            if data_len > compression_threshold.0 as usize {
                // Get the data to compress
                let data_to_compress = &self.buf[start_len..];

                // Clear the compression buffer
                self.compress_buf.clear();

                // Compute the maximum size of compressed data
                let max_compressed_size = compressor.zlib_compress_bound(data_to_compress.len());

                // Ensure compress_buf has enough capacity
                self.compress_buf.resize(max_compressed_size, 0);

                // Compress the data
                let compressed_size = compressor
                    .zlib_compress(data_to_compress, &mut self.compress_buf)
                    .map_err(|e| PacketEncodeError::CompressionFailed(e.to_string()))?;

                // Resize compress_buf to actual compressed size
                self.compress_buf.resize(compressed_size, 0);

                let data_len_size = VarInt(data_len as i32).written_size();

                let packet_len = data_len_size + compressed_size;

                if packet_len >= MAX_PACKET_SIZE {
                    return Err(PacketEncodeError::TooLong(packet_len));
                }

                self.buf.truncate(start_len);

                VarInt(packet_len as i32).encode(&mut self.buf);
                VarInt(data_len as i32).encode(&mut self.buf);
                self.buf.extend_from_slice(&self.compress_buf);
            } else {
                let data_len_size = 1;
                let packet_len = data_len_size + data_len;

                if packet_len >= MAX_PACKET_SIZE {
                    Err(PacketEncodeError::TooLong(packet_len))?
                }

                let packet_len_size = VarInt(packet_len as i32).written_size();

                let data_prefix_len = packet_len_size + data_len_size;

                self.buf.put_bytes(0, data_prefix_len);
                self.buf
                    .copy_within(start_len..start_len + data_len, start_len + data_prefix_len);

                let mut front = &mut self.buf[start_len..];

                VarInt(packet_len as i32).encode(&mut front);
                // Zero for no compression on this packet.
                VarInt(0).encode(&mut front);
            }

            return Ok(());
        }

        let packet_len = data_len;

        if packet_len >= MAX_PACKET_SIZE {
            Err(PacketEncodeError::TooLong(packet_len))?
        }

        let packet_len_size = VarInt(packet_len as i32).written_size();

        self.buf.put_bytes(0, packet_len_size);
        self.buf
            .copy_within(start_len..start_len + data_len, start_len + packet_len_size);

        let mut front = &mut self.buf[start_len..];
        VarInt(packet_len as i32).encode(&mut front);
        Ok(())
    }

    /// Enable encryption for taking all packets buffer `
    pub fn set_encryption(&mut self, key: Option<&[u8; 16]>) {
        if let Some(key) = key {
            assert!(self.cipher.is_none(), "encryption is already enabled");

            self.cipher = Some(Cipher::new_from_slices(key, key).expect("invalid key"));
        } else {
            assert!(self.cipher.is_some(), "encryption is disabled");

            self.cipher = None;
        }
    }

    /// Enables or disables Zlib compression.
    ///
    /// If `compression` is `Some`, compression is enabled with the given `threshold`
    /// for triggering compression and the specified `level`. If `compression` is
    /// `None`, compression is disabled.
    ///
    /// # Errors
    ///
    /// Returns an `CompressionLevelError` if an invalid compression level is provided.
    pub fn set_compression(
        &mut self,
        compression: Option<(CompressionThreshold, CompressionLevel)>,
    ) -> Result<(), CompressionLevelError> {
        match compression {
            Some((threshold, level)) => {
                let level =
                    CompressionLvl::new(level.0 as i32).map_err(|_| CompressionLevelError)?;
                self.compression = Some((Compressor::new(level), threshold));
            }
            None => {
                self.compression = None;
            }
        }
        Ok(())
    }

    /// Encrypts the data in the internal buffer and returns it as a `BytesMut`.
    ///
    /// If a cipher is set, the data is encrypted in-place using block cipher encryption.
    /// The buffer is processed in chunks of the cipher's block size. If the buffer's
    /// length is not a multiple of the block size, the last partial block is *not* encrypted.
    /// It's important to ensure that the data being encrypted is padded appropriately
    /// beforehand if necessary.
    ///
    /// If no cipher is set, the buffer is returned as is.
    pub fn take(&mut self) -> BytesMut {
        if let Some(cipher) = &mut self.cipher {
            for chunk in self.buf.chunks_mut(Cipher::block_size()) {
                let gen_arr = GenericArray::from_mut_slice(chunk);
                cipher.encrypt_block_mut(gen_arr);
            }
        }

        self.buf.split()
    }
}

#[derive(Error, Debug)]
#[error("Invalid compression Level")]
pub struct CompressionLevelError;

/// Errors that can occur during packet encoding.
#[derive(Error, Debug)]
pub enum PacketEncodeError {
    #[error("Packet exceeds maximum length: {0}")]
    TooLong(usize),
    #[error("Compression failed {0}")]
    CompressionFailed(String),
}