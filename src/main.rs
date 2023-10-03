use std::path::PathBuf;
use clap::Parser;

mod read_ext;
mod byte_ext;

mod blend;
use blend::*;

mod input;
use input::*;

mod output;
use output::*;

use crate::read_ext::ReadExt;

/// A program to explode blend files into their many parts.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about)] // Read from `Cargo.toml`
struct Blend2Zip {
    /// The `.blend`-file to explode into parts.
    /// 
    /// By specifying `-` as FILE, reading from STDIN is supported.
    #[arg(value_name = "FILE")]
    src: PathBuf,
    
    /// Where to write the exploded blend-file parts to.
    /// 
    /// The file-extension determines the output format:
    /// 
    /// - `zip` writes a ZIP-archive.
    /// 
    /// - `tar` writes a tape-archive.
    /// 
    /// By specifying `-` as OUT, writing to STDOUT as TAR is supported.
    #[arg(value_name = "OUT")]
    dst: PathBuf,
    
    /// Exclude files from being emitted via globs.
    /// 
    /// Uses <https://crates.io/crates/globset> internally.
    #[arg(short='x',long="exclude",value_name = "GLOB")]
    excludes: Vec<String>,
}

fn main() {
    let args = Blend2Zip::parse();
    run(args).unwrap();
}

fn run(args: Blend2Zip) -> std::io::Result<()> {
    let mut input = select_input(&args.src);
    let mut output = select_output(&args.dst);
    
    if let Some(globber) = build_globber(args.excludes) {
        output = Box::new(OutputGlobber {
            globset: globber,
            output,
        });
    }
    
    
    let blend = read_header(&mut input)?;
    let blend_info = format!("{blend}");
    
    output.write_file(
        "blend.txt",
        blend_info.len() as u64,
        &mut std::io::Cursor::new(blend_info)
    ).unwrap();
    
    loop {
        let chunk_head = match read_chunk_header(&blend, &mut input) {
            Ok(chunk) => chunk,
            Err(err) => panic!("Failed to read chunk header: {err:?}")
        };
        
        //eprintln!("Parsed chunk: {chunk_head}");
        
        if chunk_head.code == b"DNA1" {
            let dna1 = input.read_exact_buffer(chunk_head.size as usize)?;
            
            output.write_file(
                "DNA1.bin",
                dna1.len() as u64,
                &mut std::io::Cursor::new(&dna1)
            ).unwrap();
            
            // Time to parse DNA1!
            read_dna1(&blend, dna1, output.as_mut())?;
            continue;
        }
        
        let path = format!("{}/0x{:X?}", chunk_head.code, chunk_head.addr);
        
        output.write_file(
            &format!("{path}.bin"),
            chunk_head.size as u64,
            &mut input.take_borrowed(chunk_head.size as usize)
        ).unwrap();
        
        let meta = format!("code\t{}\nsize\t0x{:X?}\naddr\t0x{:X?}\nsdna\t0x{:X?}\ncount\t{}\n"
            , chunk_head.code
            , chunk_head.size
            , chunk_head.addr
            , chunk_head.sdna
            , chunk_head.count
        );
        
        output.write_file(
            &format!("{path}.txt"),
            meta.len() as u64,
            &mut std::io::Cursor::new(meta)
        ).unwrap();
        
        if chunk_head.code == b"ENDB" {
            eprintln!("Reached ENDB chunk.");
            break;
        }
    }
    
    output.finish();
    Ok(())
}

fn build_globber(excludes: Vec<String>) -> Option<globset::GlobSet> {
    if excludes.is_empty() {
        return None
    }
    
    use globset::{Glob, GlobSetBuilder};
    let mut builder = GlobSetBuilder::new();
    
    for glob in excludes {
        let glob = Glob::new(&glob).expect("failed to build glob");
        builder.add(glob);
    }
    
    Some(builder.build().expect("failed to build globset"))
}
