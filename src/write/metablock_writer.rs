use crate::compress_threads::ParallelCompressor;
use crate::pool;
use std::convert::TryInto;
use std::sync::Arc;
use std::{io, mem};
use zerocopy::AsBytes;

pub struct MetablockWriter {
    compressor: Option<Arc<ParallelCompressor>>,
    output: Vec<u8>,
    current_block: Vec<u8>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ItemPosition {
    pub block_start: u32,
    pub uncompressed_offset: u16,
}

impl MetablockWriter {
    pub fn new(compressor: Option<Arc<ParallelCompressor>>) -> Self {
        Self {
            compressor,
            output: Vec::new(),
            current_block: pool::block().detach(),
        }
    }

    fn position(&self) -> ItemPosition {
        ItemPosition {
            block_start: self.output.len().try_into().unwrap(),
            uncompressed_offset: self.current_block.len().try_into().unwrap(),
        }
    }

    pub async fn write<T: AsBytes>(&mut self, item: &T) -> io::Result<ItemPosition> {
        let position = self.position();
        let item = item.as_bytes();
        let remaining_len = repr::metablock::SIZE - self.current_block.len();
        if remaining_len < item.len() {
            let (head, tail) = item.split_at(remaining_len);
            self.current_block.extend_from_slice(head);
            self.flush().await?;
            self.current_block.extend_from_slice(tail);
        } else {
            self.current_block.extend_from_slice(item);
        }
        Ok(position)
    }

    pub async fn finish(mut self) -> io::Result<Vec<u8>> {
        self.flush().await?;
        Ok(mem::take(&mut self.output))
    }

    async fn flush(&mut self) -> io::Result<()> {
        if let Some(compressor) = &self.compressor {
            let block = mem::replace(&mut self.current_block, pool::block().detach());
            let result = compressor.compress(block).await?;

            Self::write_output(&mut self.output, &result.data, result.compressed);
        } else {
            Self::write_output(&mut self.output, &self.current_block, false);
            self.current_block.clear();
        }

        Ok(())
    }

    fn write_output(output: &mut Vec<u8>, data: &[u8], compressed: bool) {
        let header = repr::metablock::Header::new(data.len().try_into().unwrap(), compressed);
        let header_bytes = header.as_bytes();

        output.reserve(header_bytes.len() + data.len());
        output.extend_from_slice(header_bytes);
        output.extend_from_slice(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compression::Compressor;
    use zerocopy::AsBytes;

    fn pos(pos: ItemPosition) -> (u32, u16) {
        (pos.block_start, pos.uncompressed_offset)
    }

    #[test]
    fn simple() {
        #[derive(AsBytes)]
        #[repr(C)]
        struct BigT {
            data: [u8; 1000],
        }

        let compressor =
            ParallelCompressor::new(1, Compressor::new(crate::compression::Kind::default()));
        let compressor = Arc::new(compressor);

        let mut writer = MetablockWriter::new(Some(compressor));

        futures::executor::block_on(async {
            let big_t = BigT { data: [0; 1000] };
            // Write 9 * 1000 bytes so the next one will start in the second metablock
            for i in 0..9 {
                let position = writer.write(&big_t).await.unwrap();
                assert_eq!(pos(position), (0, i * 1000));
            }

            // This will start in the second metablock. The first metablock should compress well
            let position = writer.write(&big_t).await.unwrap();
            assert!((1..400).contains(&position.block_start));
            assert_eq!(
                usize::from(position.uncompressed_offset),
                (9 * 1000) % repr::metablock::SIZE
            );

            let result = writer.finish().await.unwrap();
        });
    }
}
