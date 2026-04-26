use uuid::Uuid;

use super::NONE_STRING_LEN;

pub(super) struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BinaryReader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(super) fn finish(&self) -> anyhow::Result<()> {
        if self.offset == self.bytes.len() {
            return Ok(());
        }
        Err(anyhow::anyhow!("Trailing bytes in live chat binary frame"))
    }

    fn read_exact(&mut self, len: usize) -> anyhow::Result<&'a [u8]> {
        let end = self.offset.saturating_add(len);
        if end > self.bytes.len() {
            return Err(anyhow::anyhow!("Truncated live chat binary frame"));
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    pub(super) fn read_u8(&mut self) -> anyhow::Result<u8> {
        let bytes = self.read_exact(1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self) -> anyhow::Result<u16> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub(super) fn read_u64(&mut self) -> anyhow::Result<u64> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub(super) fn read_uuid(&mut self) -> anyhow::Result<Uuid> {
        let bytes = self.read_exact(16)?;
        let mut raw = [0u8; 16];
        raw.copy_from_slice(bytes);
        Ok(Uuid::from_bytes(raw))
    }

    pub(super) fn read_string(&mut self) -> anyhow::Result<String> {
        let len = self.read_u16()?;
        if len == NONE_STRING_LEN {
            return Err(anyhow::anyhow!(
                "Unexpected null string in live chat binary frame"
            ));
        }
        let bytes = self.read_exact(len as usize)?;
        String::from_utf8(bytes.to_vec()).map_err(|e| anyhow::anyhow!(e))
    }
}
