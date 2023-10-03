use std::io::{Read, Result};

pub trait ReadExt: Read {
    fn read_byte(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }
    
    fn read_exact_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut buf = [0; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
    
    fn read_exact_buffer(&mut self, limit: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; limit];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
    
    fn read_cstr(&mut self) -> Result<std::ffi::CString> {
        let mut buf = [0u8; 256];
        let mut len = 0;
        
        loop {
            let byte = self.read_byte()?;
            buf[len] = byte;
            len += 1;
            
            if byte == 0 {
                break;
            }
        }
        
        Ok(std::ffi::CStr::from_bytes_with_nul(&buf[..len]).unwrap().to_owned())
    }
    
    fn take_borrowed<'b, 'r: 'b>(&'r mut self, len: usize) -> BorrowedTake<'b, Self> {
        BorrowedTake {
            from: self,
            rem: len
        }
    }
    
}

impl<R> ReadExt for R where R: Read {}

pub struct BorrowedTake<'r, R: std::io::Read + ?Sized> {
    from: &'r mut R,
    rem: usize,
}

impl<'r, R: std::io::Read> std::io::Read for BorrowedTake<'r, R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.rem == 0 {
            return Ok(0)
        }
        
        let nom = buf.len().min(self.rem);
        let got = self.from.read(&mut buf[..nom])?;
        self.rem -= got;
        
        Ok(got)
    }
}

