use std::io;
use std::io::Write;

#[derive(Debug)]
pub struct WritableBuffer {
    buf: String,
}

impl WritableBuffer {
    pub fn new() -> WritableBuffer {
        WritableBuffer { buf: String::new() }
    }
}

impl From<WritableBuffer> for String {
    fn from(wb: WritableBuffer) -> Self {
        wb.buf
    }
}

impl Write for WritableBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.buf.push_str(s);
                Ok(buf.len())
            }
            Err(_) => Err(io::ErrorKind::InvalidInput.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_utf8_and_appends() {
        let mut wb = WritableBuffer::new();
        assert_eq!(wb.write(b"hello").unwrap(), 5);
        assert_eq!(wb.write(b" world").unwrap(), 6);
        assert_eq!(String::from(wb), "hello world");
    }

    #[test]
    fn writes_multibyte_utf8() {
        let mut wb = WritableBuffer::new();
        let s = "héllo 🌍".as_bytes();
        assert_eq!(wb.write(s).unwrap(), s.len());
        assert_eq!(String::from(wb), "héllo 🌍");
    }

    #[test]
    fn rejects_invalid_utf8() {
        let mut wb = WritableBuffer::new();
        let err = wb.write(&[0xff, 0xfe, 0xfd]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
