use anyhow::anyhow;
use scroll::Pread;

/// Trait that must be implemented to access the RTT region on the target.
pub trait RttInterface {
    fn core_id(&self) -> usize;

    /// Does this interface support native 64-bit wide accesses
    ///
    /// If false all 64-bit operations may be split into 32 or 8 bit operations.
    /// Most callers will not need to pivot on this but it can be useful for
    /// picking the fastest bulk data transfer method.
    fn supports_native_64bit_access(&mut self) -> bool;

    /// Read a 64bit word of at `address`.
    ///
    /// The address where the read should be performed at has to be word aligned.
    /// Returns `MemoryError::MemoryNotAligned` if this does not hold true.
    fn read_word_64(&mut self, address: u64) -> Result<u64, MemoryError>;

    /// Read a 32bit word of at `address`.
    ///
    /// The address where the read should be performed at has to be word aligned.
    /// Returns [`MemoryError::MemoryNotAligned`] if this does not hold true.
    fn read_word_32(&mut self, address: u64) -> Result<u32, MemoryError>;

    /// Read an 8bit word of at `address`.
    fn read_word_8(&mut self, address: u64) -> Result<u8, MemoryError>;

    /// Read a block of 64bit words at `address`.
    ///
    /// The number of words read is `data.len()`.
    /// The address where the read should be performed at has to be word aligned.
    /// Returns [`MemoryError::MemoryNotAligned`] if this does not hold true.
    fn read_64(&mut self, address: u64, data: &mut [u64]) -> Result<(), MemoryError>;

    /// Read a block of 32bit words at `address`.
    ///
    /// The number of words read is `data.len()`.
    /// The address where the read should be performed at has to be word aligned.
    /// Returns [`MemoryError::MemoryNotAligned`] if this does not hold true.
    fn read_32(&mut self, address: u64, data: &mut [u32]) -> Result<(), MemoryError>;

    /// Read a block of 8bit words at `address`.
    fn read_8(&mut self, address: u64, data: &mut [u8]) -> Result<(), MemoryError>;

    /// Reads bytes using 64 bit memory access. Address must be 64 bit aligned
    /// and data must be an exact multiple of 8.
    fn read_mem_64bit(&mut self, address: u64, data: &mut [u8]) -> Result<(), MemoryError> {
        // Default implementation uses `read_64`, then converts u64 values back
        // to bytes. Assumes target is little endian. May be overridden to
        // provide an implementation that avoids heap allocation and endian
        // conversions. Must be overridden for big endian targets.
        if data.len() % 8 != 0 {
            return Err(MemoryError::Other(anyhow!(
                "Call to read_mem_64bit with data.len() not a multiple of 8"
            )));
        }
        let mut buffer = vec![0u64; data.len() / 8];
        self.read_64(address, &mut buffer)?;
        for (bytes, value) in data.chunks_exact_mut(8).zip(buffer.iter()) {
            bytes.copy_from_slice(&u64::to_le_bytes(*value));
        }
        Ok(())
    }

    /// Reads bytes using 32 bit memory access. Address must be 32 bit aligned
    /// and data must be an exact multiple of 4.
    fn read_mem_32bit(&mut self, address: u64, data: &mut [u8]) -> Result<(), MemoryError> {
        // Default implementation uses `read_32`, then converts u32 values back
        // to bytes. Assumes target is little endian. May be overridden to
        // provide an implementation that avoids heap allocation and endian
        // conversions. Must be overridden for big endian targets.
        if data.len() % 4 != 0 {
            return Err(MemoryError::Other(anyhow!(
                "Call to read_mem_32bit with data.len() not a multiple of 4"
            )));
        }
        let mut buffer = vec![0u32; data.len() / 4];
        self.read_32(address, &mut buffer)?;
        for (bytes, value) in data.chunks_exact_mut(4).zip(buffer.iter()) {
            bytes.copy_from_slice(&u32::to_le_bytes(*value));
        }
        Ok(())
    }

    /// Read data from `address`.
    ///
    /// This function tries to use the fastest way of reading data, so there is no
    /// guarantee which kind of memory access is used. The function might also read more
    /// data than requested, e.g. when the start address is not aligned to a 32-bit boundary.
    ///
    /// For more control, the `read_x` functions, e.g. [`MemoryInterface::read_32()`], can be
    /// used.
    ///
    ///  Generally faster than `read_8`.
    fn read(&mut self, address: u64, data: &mut [u8]) -> Result<(), MemoryError> {
        if self.supports_native_64bit_access() && address % 8 == 0 && data.len() % 8 == 0 {
            // Avoid heap allocation and copy if we don't need it.
            self.read_mem_64bit(address, data)?;
        } else if address % 4 == 0 && data.len() % 4 == 0 {
            // Avoid heap allocation and copy if we don't need it.
            self.read_mem_32bit(address, data)?;
        } else {
            let start_extra_count = (address % 4) as usize;
            let mut buffer = vec![0u8; (start_extra_count + data.len() + 3) / 4 * 4];
            self.read_mem_32bit(address - start_extra_count as u64, &mut buffer)?;
            data.copy_from_slice(&buffer[start_extra_count..start_extra_count + data.len()]);
        }
        Ok(())
    }

    /// Write a 64bit word at `address`.
    ///
    /// The address where the write should be performed at has to be word aligned.
    /// Returns [`MemoryError::MemoryNotAligned`] if this does not hold true.
    fn write_word_64(&mut self, address: u64, data: u64) -> Result<(), MemoryError>;

    /// Write a 32bit word at `address`.
    ///
    /// The address where the write should be performed at has to be word aligned.
    /// Returns [`MemoryError::MemoryNotAligned`] if this does not hold true.
    fn write_word_32(&mut self, address: u64, data: u32) -> Result<(), MemoryError>;

    /// Write an 8bit word at `address`.
    fn write_word_8(&mut self, address: u64, data: u8) -> Result<(), MemoryError>;

    /// Write a block of 64bit words at `address`.
    ///
    /// The number of words written is `data.len()`.
    /// The address where the write should be performed at has to be word aligned.
    /// Returns [`MemoryError::MemoryNotAligned`] if this does not hold true.
    fn write_64(&mut self, address: u64, data: &[u64]) -> Result<(), MemoryError>;

    /// Write a block of 32bit words at `address`.
    ///
    /// The number of words written is `data.len()`.
    /// The address where the write should be performed at has to be word aligned.
    /// Returns [`MemoryError::MemoryNotAligned`] if this does not hold true.
    fn write_32(&mut self, address: u64, data: &[u32]) -> Result<(), MemoryError>;

    /// Write a block of 8bit words at `address`.
    fn write_8(&mut self, address: u64, data: &[u8]) -> Result<(), MemoryError>;

    /// Writes bytes using 64 bit memory access. Address must be 64 bit aligned
    /// and data must be an exact multiple of 8.
    fn write_mem_64bit(&mut self, address: u64, data: &[u8]) -> Result<(), MemoryError> {
        // Default implementation uses `write_64`, then converts u64 values back
        // to bytes. Assumes target is little endian. May be overridden to
        // provide an implementation that avoids heap allocation and endian
        // conversions. Must be overridden for big endian targets.
        if data.len() % 8 != 0 {
            return Err(MemoryError::Other(anyhow!(
                "Call to read_mem_64bit with data.len() not a multiple of 8"
            )));
        }
        let mut buffer = vec![0u64; data.len() / 8];
        for (bytes, value) in data.chunks_exact(8).zip(buffer.iter_mut()) {
            *value = bytes
                .pread_with(0, scroll::LE)
                .expect("an u64 - this is a bug, please report it");
        }

        self.write_64(address, &buffer)?;
        Ok(())
    }

    /// Writes bytes using 32 bit memory access. Address must be 32 bit aligned
    /// and data must be an exact multiple of 8.
    fn write_mem_32bit(&mut self, address: u64, data: &[u8]) -> Result<(), MemoryError> {
        // Default implementation uses `write_32`, then converts u32 values back
        // to bytes. Assumes target is little endian. May be overridden to
        // provide an implementation that avoids heap allocation and endian
        // conversions. Must be overridden for big endian targets.
        if data.len() % 4 != 0 {
            return Err(MemoryError::Other(anyhow!(
                "Call to read_mem_32bit with data.len() not a multiple of 4"
            )));
        }
        let mut buffer = vec![0u32; data.len() / 4];
        for (bytes, value) in data.chunks_exact(4).zip(buffer.iter_mut()) {
            *value = bytes
                .pread_with(0, scroll::LE)
                .expect("an u32 - this is a bug, please report it");
        }

        self.write_32(address, &buffer)?;
        Ok(())
    }

    /// Write a block of 8bit words at `address`. May use 64 bit memory access,
    /// so should only be used if reading memory locations that don't have side
    /// effects. Generally faster than [`MemoryInterface::write_8`].
    ///
    /// If the target does not support 8-bit aligned access, and `address` is not
    /// aligned on a 32-bit boundary, this function will return a [`MemoryError::MemoryNotAligned`] error.
    fn write(&mut self, address: u64, data: &[u8]) -> Result<(), MemoryError> {
        let len = data.len();
        let start_extra_count = 4 - (address % 4) as usize;
        let end_extra_count = (len - start_extra_count) % 4;
        let inbetween_count = len - start_extra_count - end_extra_count;
        assert!(start_extra_count < 4);
        assert!(end_extra_count < 4);
        assert!(inbetween_count % 4 == 0);

        // If we do not have 32 bit aligned access we first check that we can do 8 bit aligned access on this platform.
        // If we cannot we throw an error.
        // If we can we read the first n < 4 bytes up until the word aligned address that comes next.
        if address % 4 != 0 || len % 4 != 0 {
            // If we do not support 8 bit transfers we have to bail because we can only do 32 bit word aligned transers.
            if !self.supports_8bit_transfers()? {
                return Err(MemoryError::MemoryNotAligned {
                    address,
                    alignment: 4,
                });
            }

            // We first do an 8 bit write of the first < 4 bytes up until the 4 byte aligned boundary.
            self.write_8(address, &data[..start_extra_count])?;
        }

        let mut buffer = vec![0u32; inbetween_count / 4];
        for (bytes, value) in data.chunks_exact(4).zip(buffer.iter_mut()) {
            *value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        self.write_32(address, &buffer)?;

        // We read the remaining bytes that we did not read yet which is always n < 4.
        if end_extra_count > 0 {
            self.write_8(address, &data[..start_extra_count])?;
        }

        Ok(())
    }

    /// Returns whether the current platform supports native 8bit transfers.
    fn supports_8bit_transfers(&self) -> Result<bool, MemoryError>;

    /// Flush any outstanding operations.
    ///
    /// For performance, debug probe implementations may choose to batch writes;
    /// to assure that any such batched writes have in fact been issued, `flush`
    /// can be called.  Takes no arguments, but may return failure if a batched
    /// operation fails.
    fn flush(&mut self) -> Result<(), MemoryError>;
}

#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    /// Any other error occurred.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    /// Unaligned memory access
    #[error("Alignment error")]
    MemoryNotAligned {
        /// The address of the register.
        address: u64,
        /// The required alignment in bytes (address increments).
        alignment: usize,
    },
}
