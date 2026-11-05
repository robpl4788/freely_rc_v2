use crate::hal::register_extras::RegisterExtras;

const CHANNEL_ENABLE_INDEX: u32 = 0;
const TRANSFER_COMPLETE_FLAG_OFFSET: u32 = 1;
const HALF_TRANSFER_FLAG_OFFSET: u32 = 2;
const TRANSFER_ERROR_FLAG_OFFSET: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DmaChannelId {
    Channel1 = 0,
    Channel2 = 1,
    Channel3 = 2,
    Channel4 = 3,
    Channel5 = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaError {
    ChannelBusy,
    EmptyBuffer,
    BufferTooLong,
    MisalignedAddress,
}

pub struct Dma1 {
    register_block: &'static at32f4xx_pac::at32f421::dma1::RegisterBlock,
    taken_channels: u8,
}

impl Dma1 {
    pub(super) fn new() -> Self {
        Self {
            register_block: unsafe { &*at32f4xx_pac::at32f421::DMA1::ptr() },
            taken_channels: 0,
        }
    }

    pub fn take_channel(&mut self, id: DmaChannelId) -> Option<DmaChannel> {
        let channel_mask = 1 << id as u8;
        if self.taken_channels & channel_mask != 0 {
            return None;
        }
        self.taken_channels |= channel_mask;

        Some(DmaChannel {
            register_block: self.register_block,
            id,
        })
    }
}

pub struct DmaTransfer<'channel, 'buffer> {
    channel: &'channel mut DmaChannel,
    destination: Option<&'buffer mut [u16]>,
}

pub struct DmaChannel {
    register_block: &'static at32f4xx_pac::at32f421::dma1::RegisterBlock,
    id: DmaChannelId,
}

impl DmaChannel {
    /// # Safety
    /// The peripheral address must be a valid DMA source that supports 16-bit
    /// reads, and the destination must be DMA-accessible memory. Do not access
    /// the destination until the returned transfer has been stopped.
    pub unsafe fn start_peripheral_to_memory<'channel, 'buffer>(
        &'channel mut self,
        peripheral_address: *const u16,
        destination: &'buffer mut [u16],
        circular: bool,
    ) -> Result<DmaTransfer<'channel, 'buffer>, DmaError> {
        if self.registers().ctrl().read_bit(CHANNEL_ENABLE_INDEX) {
            return Err(DmaError::ChannelBusy);
        }
        if destination.is_empty() {
            return Err(DmaError::EmptyBuffer);
        }
        if destination.len() > u16::MAX as usize {
            return Err(DmaError::BufferTooLong);
        }
        if (peripheral_address as usize | destination.as_ptr() as usize) & 1 != 0 {
            return Err(DmaError::MisalignedAddress);
        }

        self.clear_flags();
        self.registers()
            .paddr()
            .write(|w| unsafe { w.bits(peripheral_address as usize as u32) });
        self.registers()
            .maddr()
            .write(|w| unsafe { w.bits(destination.as_mut_ptr() as usize as u32) });
        self.registers()
            .dtcnt()
            .write(|w| unsafe { w.bits(destination.len() as u16) });

        let control = (1 << 1)
            | (1 << 7)
            | (1 << 8)
            | (1 << 10)
            | (0b10 << 12)
            | (u32::from(circular) << 5)
            | (1 << CHANNEL_ENABLE_INDEX);
        self.registers()
            .ctrl()
            .write(|w| unsafe { w.bits(control) });
        Ok(DmaTransfer {
            channel: self,
            destination: Some(destination),
        })
    }

    pub fn is_transfer_complete(&self) -> bool {
        self.status_bit(TRANSFER_COMPLETE_FLAG_OFFSET)
    }

    pub fn is_half_transfer_complete(&self) -> bool {
        self.status_bit(HALF_TRANSFER_FLAG_OFFSET)
    }

    pub fn has_transfer_error(&self) -> bool {
        self.status_bit(TRANSFER_ERROR_FLAG_OFFSET)
    }

    pub fn stop(&mut self) {
        self.registers().ctrl().set_bit(false, CHANNEL_ENABLE_INDEX);
        while self.registers().ctrl().read_bit(CHANNEL_ENABLE_INDEX) {}
    }

    fn registers(&self) -> &at32f4xx_pac::at32f421::dma1::Channel {
        self.register_block.channel(self.id as usize)
    }

    fn clear_flags(&self) {
        let mask = 0b1111 << (u32::from(self.id as u8) * 4);
        self.register_block.clr().write(|w| unsafe { w.bits(mask) });
    }

    fn status_bit(&self, flag_offset: u32) -> bool {
        let bit = u32::from(self.id as u8) * 4 + flag_offset;
        self.register_block.sts().read().bits() & (1 << bit) != 0
    }
}

impl Drop for DmaChannel {
    fn drop(&mut self) {
        self.stop();
    }
}

impl<'channel, 'buffer> DmaTransfer<'channel, 'buffer> {
    pub fn is_complete(&self) -> bool {
        self.channel.is_transfer_complete()
    }

    pub fn is_half_complete(&self) -> bool {
        self.channel.is_half_transfer_complete()
    }

    pub fn has_error(&self) -> bool {
        self.channel.has_transfer_error()
    }

    pub fn stop(mut self) -> &'buffer mut [u16] {
        self.channel.stop();
        self.channel.clear_flags();
        self.destination.take().expect("DMA destination is present")
    }
}

impl Drop for DmaTransfer<'_, '_> {
    fn drop(&mut self) {
        if self.destination.is_some() {
            self.channel.stop();
        }
    }
}
