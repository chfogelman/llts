use std::io;
use std::ops::ControlFlow;

use zerocopy::{IntoBytes, network_endian as ne};

use crate::messages::{Handler, dispatch};

pub struct BinFileFramer<R: io::Read, H: Handler> {
    reader:  R,
    buf:     [u8; 128],
    handler: H,
}

impl<R: io::Read, H: Handler> BinFileFramer<R, H> {
    pub fn new(reader: R, handler: H) -> Self {
        Self {
            reader,
            buf: [0; 128],
            handler,
        }
    }

    pub fn poll(&mut self) -> io::Result<ControlFlow<()>> {
        let mut len = ne::U16::new(0);
        let mut idx = 0;
        while idx < 2 {
            let bytes_read = self.reader.read(&mut len.as_mut_bytes()[idx..])?;
            if bytes_read == 0 {
                if idx == 0 {
                    return Ok(ControlFlow::Break(()));
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Unexpected EOF while parsing msg length",
                    ));
                }
            }
            idx += bytes_read;
        }
        let len = len.get() as usize;
        if len == 0 {
            return Ok(ControlFlow::Break(()));
        }
        self.reader.read_exact(&mut self.buf[..len])?;
        dispatch(&mut self.handler, &self.buf[..len]);
        Ok(ControlFlow::Continue(()))
    }
}
