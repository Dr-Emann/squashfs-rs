use crate::SparseRead;
use std::fs::File;
use std::io;
use std::io::{Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};

const SEEK_DATA: libc::c_int = 4;

static SEEK_DATA_BROKEN: AtomicBool = AtomicBool::new(false);

impl SparseRead for File {
    fn skip_hole(&mut self) -> std::io::Result<u64> {
        if SEEK_DATA_BROKEN.load(Ordering::Relaxed) {
            return Ok(0);
        }

        let start = self.stream_position()?;
        let offset: libc::off_t = match start.try_into() {
            Ok(start) => start,
            Err(_) => return Ok(0),
        };
        let res = unsafe { libc::lseek(self.as_raw_fd(), offset, SEEK_DATA) };
        if res < 0 {
            let err = io::Error::last_os_error();
            match err.raw_os_error() {
                Some(errno) if errno == libc::EINVAL => {
                    SEEK_DATA_BROKEN.store(true, Ordering::Relaxed);
                    Ok(0)
                }
                Some(errno) if errno == libc::ENXIO => {
                    let end = self.seek(SeekFrom::End(0))?;
                    Ok(end - start)
                }
                _ => Err(err),
            }
        } else {
            Ok(res as u64 - start)
        }
    }
}
