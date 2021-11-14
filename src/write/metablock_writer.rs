use crate::compress_threads::ParallelCompressor;
use crate::pool;
use std::convert::TryInto;
use std::mem;
use std::sync::Arc;
use zerocopy::AsBytes;

#[derive(Debug, Default)]
pub struct MetablockWriter {
    compressor: Option<Arc<ParallelCompressor>>,
    output: Vec<u8>,
    current_block: Vec<u8>,
}

impl MetablockWriter {
    pub fn new(compressor: Option<Arc<ParallelCompressor>>) -> Self {
        Self::with_capacity(compressor, 0)
    }

    pub fn with_capacity(compressor: Option<Arc<ParallelCompressor>>, cap: usize) -> Self {
        Self {
            compressor,
            output: Vec::with_capacity(cap),
            current_block: pool::block().detach(),
        }
    }

    pub fn position(&self) -> repr::metablock::Ref {
        repr::metablock::Ref::new(
            self.output.len().try_into().unwrap(),
            self.current_block.len().try_into().unwrap(),
        )
    }

    pub async fn write<T: AsBytes>(&mut self, item: &T) {
        self.write_raw(item.as_bytes()).await
    }

    pub async fn write_raw(&mut self, mut data: &[u8]) {
        while repr::metablock::SIZE - self.current_block.len() < data.len() {
            let (head, tail) = data.split_at(repr::metablock::SIZE - self.current_block.len());
            self.current_block.extend_from_slice(head);
            self.flush().await;
            data = tail;
        }
        self.current_block.extend_from_slice(data);
    }

    pub async fn finish(mut self) -> Vec<u8> {
        self.flush().await;
        mem::take(&mut self.output)
    }

    async fn flush(&mut self) {
        if let Some(compressor) = &self.compressor {
            let block = mem::replace(&mut self.current_block, pool::block().detach());
            let result = compressor.compress(block).await;

            Self::write_output(&mut self.output, &result.data, result.compressed);
        } else {
            Self::write_output(&mut self.output, &self.current_block, false);
            self.current_block.clear();
        }
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

    fn pos(pos: repr::metablock::Ref) -> (u32, u16) {
        (pos.block_start(), pos.start_offset())
    }

    #[test]
    fn simple() {
        #[derive(AsBytes)]
        #[repr(C)]
        struct BigT {
            data: [u8; 1000],
        }

        let compressor = ParallelCompressor::with_threads(
            Compressor::new(crate::compression::Kind::default()),
            1,
        );
        let compressor = Arc::new(compressor);

        let mut writer = MetablockWriter::new(Some(compressor));

        futures::executor::block_on(async {
            let big_t = BigT { data: [0; 1000] };
            // Write 9 * 1000 bytes so the next one will start in the second metablock
            for i in 0..9 {
                let position = writer.position();
                writer.write(&big_t).await;
                assert_eq!(pos(position), (0, i * 1000));
            }

            // This will start in the second metablock. The first metablock should compress well
            let position = writer.position();
            writer.write(&big_t).await;
            assert!((1..400).contains(&position.block_start()));
            assert_eq!(
                usize::from(position.start_offset()),
                (9 * 1000) % repr::metablock::SIZE
            );

            let result = writer.finish().await;
        });
    }

    #[test]
    fn giant() {
        const GIANT_SIZE: usize = repr::metablock::SIZE * 3 + 1;
        #[derive(AsBytes)]
        #[repr(C)]
        struct GiantT {
            data: [u8; GIANT_SIZE],
        }

        let mut writer = MetablockWriter::new(None);

        futures::executor::block_on(async {
            let big_t = GiantT {
                data: [0; GIANT_SIZE],
            };
            writer.write(&big_t).await;
            let position = writer.position();
            // This will start in the fourth metablock (3 metablocks before here). Each metablock has a u16 in front of it
            assert_eq!(
                u64::from(position.block_start()),
                (3 * (2 + repr::metablock::SIZE)) as u64
            );
            assert_eq!(
                position.start_offset(),
                (GIANT_SIZE % repr::metablock::SIZE) as u16
            );

            let result = writer.finish().await;
        });
    }
}
