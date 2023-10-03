use std::{io::Read, ops::Deref};
use align_address::Align;

use crate::read_ext::ReadExt;

pub fn read_header(read: &mut impl Read) -> std::io::Result<BlendHeader> {
    eprintln!("Beginning parsing of blend-file.");
    
    let blend_magic = read.read_exact_array::<7>()?;
    
    if &blend_magic != b"BLENDER" {
        panic!("File is not a blend-file: Magic bytes dont match `BLENDER`");
    }
    
    let blend_usize = match read.read_byte()? {
        b'_' => BlendUsize::U32,
        b'-' => BlendUsize::U64,
        b => panic!("Blend-file header has invalid pointer-size: {b:?}")
    };
    
    let blend_endian = match read.read_byte()? {
        b'v' => BlendEndian::LE,
        b'V' => BlendEndian::BE,
        b => panic!("Blend-file header has invalid endianess: {b:?}")
    };
    
    let blend_version = read.read_exact_array::<3>()?;
    
    if ! blend_version.iter().all(|b| b.is_ascii_digit()) {
        panic!("Blend-file header has invalid version: {blend_version:?}");
    }
    
    let blend_version = BlendVersion {
        major: blend_version[0],
        minor: blend_version[1],
        patch: blend_version[2]
    };
    
    eprintln!("Parsed header: usize={blend_usize:?}, endian={blend_endian:?}, version={blend_version}");
    
    let blend_header = BlendHeader {
        usize: blend_usize,
        endian: blend_endian,
        version: blend_version,
    };
    
    Ok(blend_header)
}

pub fn read_chunk_header(blend: &BlendHeader, read: &mut impl Read) -> std::io::Result<BlendChunkHeader> {
    
    let chunk_code = BlendChunkCode(read.read_exact_array::<4>()?);
    
    let chunk_size = blend.endian.read_u32(read)?;
    
    let chunk_addr: u64 = match blend.usize {
        BlendUsize::U32 => blend.endian.read_u32(read)?.into(),
        BlendUsize::U64 => blend.endian.read_u64(read)?,
    };
    
    let chunk_sdna = blend.endian.read_u32(read)?;
    
    let chunk_count = blend.endian.read_u32(read)?;
    
    Ok(BlendChunkHeader {
        code: chunk_code,
        size: chunk_size,
        addr: chunk_addr,
        sdna: chunk_sdna,
        count: chunk_count
    })
}

pub fn read_dna1(blend: &BlendHeader, dna1: Vec<u8>, output: &mut dyn crate::Output) -> std::io::Result<()> {
    
    
    use crate::byte_ext::*;
    let endian = blend.endian;
    
    ///////////////////////////////////////////////////////
    
    let from = 0;
    
    if ! dna1[from..].starts_with(b"SDNA") {
        panic!("DNA1 at {from:X?} does not start with the required magic string `SDNA`");
    }
    
    let from = from + 4;
    
    ///////////////////////////////////////////////////////
    
    fn read_cstr_list(endian: BlendEndian, dna1: &[u8], from: usize) -> (usize, Vec<&std::ffi::CStr>) {
        let names_len = endian.u32(copy::<4>(&dna1[from..]));
        let mut names = Vec::with_capacity(names_len as usize);
        
        let mut from = from + 4;
        for _ in 0..names_len {
            let name = std::ffi::CStr::from_bytes_until_nul(&dna1[from..]).unwrap();
            let size = name.to_bytes_with_nul().len();
            //eprintln!("Read CStr [{from:X?}+{size}]: {name:?}");
            from += size;
            names.push(name);
        }
        
        from = from.align_up(4usize);
        
        //eprintln!("Read {}/{}: from={from:X?}", names.len(), names_len);
        
        (from, names)
    }
    
    ///////////////////////////////////////////////////////
    
    if ! dna1[from..].starts_with(b"NAME") {
        panic!("NAME-list at {from:X?} does not start with the required magic string `NAME`");
    }
    
    let from = from + 4;
    let (from, names) = read_cstr_list(endian, &dna1, from);
    
    ///////////////////////////////////////////////////////
    
    if ! dna1[from..].starts_with(b"TYPE") {
        panic!("TYPE-list at {from:X?} does not start with the required magic string `TYPE`");
    }
    
    let from = from + 4;
    let (from, types) = read_cstr_list(endian, &dna1, from);
    
    ///////////////////////////////////////////////////////
    
    if ! dna1[from..].starts_with(b"TLEN") {
        panic!("TLEN-list at {from:X?} does not start with the required magic string `TLEN`");
    }
    
    let from = from + 4;
    let (from, lengths) = {
        let mut lengths = Vec::<u16>::with_capacity(types.len());
        let mut from = from;
        for _ in 0..types.len() {
            let length = endian.u16(copy::<2>(&dna1[from..])); from += 2;
            lengths.push(length);
        }
        
        from = from.align_up(4usize);
        
        //eprintln!("Read {}/{}: from={from:X?}", lengths.len(), types.len());
        
        (from, lengths)
    };
    
    ///////////////////////////////////////////////////////
    
    if ! dna1[from..].starts_with(b"STRC") {
        panic!("STRC-list at {from:X?} does not start with the required magic string `STRC`");
    }
    
    let from = from + 4;
    
    let structs_len = endian.u32(copy::<4>(&dna1[from..]));
    let mut structs = Vec::<(u16, _)>::with_capacity(structs_len as usize);
    let mut from = from;
    
    //eprintln!("@{from:X?} structs[{structs_len}] start");
    
    for _ in 0..structs_len {
        let structtype = endian.u16(copy::<2>(&dna1[from..])); from += 2;
        let fields_len = endian.u16(copy::<2>(&dna1[from..])); from += 2;
        
        //let structname = types[structtype as usize];
        //eprintln!("@{from:X?} struct {structname:?} #{fields_len} start");
        
        let mut fields = Vec::<(u16, u16)>::with_capacity(fields_len as usize);
        
        for _ in 0..fields_len {
            let ftype = endian.u16(copy::<2>(&dna1[from..])); from += 2;
            let fname = endian.u16(copy::<2>(&dna1[from..])); from += 2;
            fields.push((ftype, fname));
            
            //eprintln!("@{from:X?} struct {structname:?} #{fields_len} field {:?} of type {:?}", names[fname as usize], types[ftype as usize]);
        }
        
        //eprintln!("@{from:X?} struct {structname:?} #{fields_len} end");
        
        structs.push((structtype, fields));
    }
    
    ///////////////////////////////////////////////////////
    
    use std::fmt::Write;
    let mut index = String::default();
    writeln!(&mut index, "sdna\tsize\tpath").unwrap();
    
    writeln!(&mut index, "-\t0x1\tbuiltin:char").unwrap();
    writeln!(&mut index, "-\t0x2\tbuiltin:short").unwrap();
    writeln!(&mut index, "-\t0x4\tbuiltin:int").unwrap();
    writeln!(&mut index, "-\t0x4\tbuiltin:float").unwrap();
    writeln!(&mut index, "-\t0x8\tbuiltin:long").unwrap();
    writeln!(&mut index, "-\t0x8\tbuiltin:double").unwrap();
    writeln!(&mut index, "-\t0x{:X?}\tbuiltin:void", blend.usize.len()).unwrap();
    
    for (sdna, (stype, fields)) in structs.iter().enumerate() {
        let ssize = lengths[*stype as usize];
        let sname = types[*stype as usize].to_string_lossy();
        
        let mut buffer = String::default();
        writeln!(&mut buffer, "# name {sname} @{stype}").unwrap();
        writeln!(&mut buffer, "# size {ssize}").unwrap();
        writeln!(&mut buffer, "# fields {}", fields.len()).unwrap();
        
        for (ftype, fname) in fields {
            let fname = names[*fname as usize].to_string_lossy();
            let ftype = types[*ftype as usize].to_string_lossy();
            writeln!(&mut buffer, "{fname}\t{ftype}").unwrap();
        }
        
        let path = format!("DNA1/{sname}.txt");
        
        match output.write_file(
            &path,
            buffer.len() as u64,
            &mut std::io::Cursor::new(&buffer)
        ) {
            Ok(()) => (),
            Err(error) => eprintln!("ERROR while writing `{path}`: {error}"),
        };
        
        writeln!(&mut index, "0x{sdna:X?}\t0x{ssize:X?}\t{path}").unwrap();
    }
    
    output.write_file(
        "DNA1.tsv",
        index.len() as u64,
        &mut std::io::Cursor::new(&index)
    )?;
    
    Ok(())
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum BlendUsize {
    U32 = b'_',
    U64 = b'-',
}

impl BlendUsize {
    pub fn len(self) -> u8 {
        match self {
            BlendUsize::U32 => 4,
            BlendUsize::U64 => 8,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum BlendEndian {
    LE = b'v',
    BE = b'V',
}

impl BlendEndian {
    pub fn read_u32(self, read: &mut impl Read) -> std::io::Result<u32> {
        Ok(match self {
            BlendEndian::LE => u32::from_le_bytes(read.read_exact_array::<4>()?),
            BlendEndian::BE => u32::from_be_bytes(read.read_exact_array::<4>()?),
        })
    }
    
    pub fn read_u64(self, read: &mut impl Read) -> std::io::Result<u64> {
        Ok(match self {
            BlendEndian::LE => u64::from_le_bytes(read.read_exact_array::<8>()?),
            BlendEndian::BE => u64::from_be_bytes(read.read_exact_array::<8>()?),
        })
    }
    
    pub fn u16(self, buf: [u8; 2]) -> u16 {
        match self {
            BlendEndian::LE => u16::from_le_bytes(buf),
            BlendEndian::BE => u16::from_be_bytes(buf),
        }
    }
    
    pub fn u32(self, buf: [u8; 4]) -> u32 {
        match self {
            BlendEndian::LE => u32::from_le_bytes(buf),
            BlendEndian::BE => u32::from_be_bytes(buf),
        }
    }
    
    // pub fn u64(self, buf: [u8; 8]) -> u64 {
    //     match self {
    //         BlendEndian::LE => u64::from_le_bytes(buf),
    //         BlendEndian::BE => u64::from_be_bytes(buf),
    //     }
    // }
}

#[derive(Debug, Clone, Copy)]
pub struct BlendVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct BlendHeader {
    pub usize: BlendUsize,
    pub endian: BlendEndian,
    pub version: BlendVersion,
}

impl std::fmt::Display for BlendHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "usize\t{:?}\nendian\t{:?}\nversion\t{}\n", self.usize, self.endian, self.version)
    }
}

impl std::fmt::Display for BlendVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}{}", self.major as char, self.minor as char, self.patch as char)
    }
}

pub struct BlendChunkCode([u8;4]);

impl std::ops::Deref for BlendChunkCode {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<&[u8]> for BlendChunkCode {
    fn eq(&self, other: &&[u8]) -> bool {
        (&self.0) == other
    }
}

impl std::fmt::Display for BlendChunkCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.deref() {
            if *byte == 0 {break}
            write!(f, "{}", *byte as char)?;
        }
        
        Ok(())
    }
}

pub struct BlendChunkHeader {
    pub code: BlendChunkCode,
    pub size: u32,
    pub addr: u64,
    pub sdna: u32,
    pub count: u32
}

impl std::fmt::Display for BlendChunkHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "code={} size={} addr=0x{:X?} sdna={} count={}", self.code, self.size, self.addr, self.sdna, self.count)
    }
}
