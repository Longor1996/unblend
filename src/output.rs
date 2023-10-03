

pub trait Output {
    
    fn write_file(
        &mut self,
        path: &str,
        size: u64,
        data: &mut dyn std::io::Read
    ) -> std::io::Result<()>;
    
    fn finish(&mut self);
}

pub type OutputBox = Box<dyn Output>;

pub struct OutputGlobber {
    pub globset: globset::GlobSet,
    pub output: OutputBox
}

impl Output for OutputGlobber {
    fn write_file(
        &mut self,
        path: &str,
        size: u64,
        data: &mut dyn std::io::Read
    ) -> std::io::Result<()> {
        
        if self.globset.is_match(path) {
            eprintln!("Voiding file `{path}` of {size} byte/s.");
            // void the file
            return std::io::copy(
                data,
                &mut std::io::sink()
            ).map(|_|());
        }
        
        
        self.output.write_file(path, size, data)
    }

    fn finish(&mut self) {
        self.output.finish();
    }
}

pub struct OutputToZip(zip_next::ZipWriter<std::fs::File>);
impl Output for OutputToZip {
    fn write_file(
        &mut self,
        path: &str,
        size: u64,
        data: &mut dyn std::io::Read
    ) -> std::io::Result<()> {
        eprintln!("Writing file `{path}` of {size} byte/s.");
        
        let options = zip_next::write::FileOptions::default();
        
        self.0.start_file(path, options)?;
        std::io::copy(data, &mut self.0)?;
        std::io::Write::flush(&mut self.0)?;
        
        Ok(())
    }
    
    fn finish(&mut self) {
        self.0.finish().unwrap();
    }
}

pub struct OutputToTar(tar::Builder<Box<dyn std::io::Write>>);
impl Output for OutputToTar {
    fn write_file(
        &mut self,
        path: &str,
        size: u64,
        data: &mut dyn std::io::Read
    ) -> std::io::Result<()> {
        eprintln!("Writing file `{path}` of {size} byte/s.");
        
        let mut header = tar::Header::new_gnu();
        header.set_size(size);
        self.0.append_data(&mut header, path, data)?;
        Ok(())
    }
    
    fn finish(&mut self) {
        self.0.finish().unwrap();
    }
}

/// Detect what type of file we should write...
pub fn select_output(dst: &std::path::PathBuf) -> OutputBox {
    if dst == std::path::Path::new("-") {
        eprintln!("Writing output to STDOUT as TAR");
        return Box::new(OutputToTar (
            tar::Builder::new(
                Box::new(
                    std::io::stdout().lock()
                )
            )
        ))
    }
    
    match &*dst.extension().expect("Unable to determine output format: OUT path has no file-extension").to_string_lossy() {
        "zip" => {
            eprintln!("Writing output to {dst:?} as ZIP");
            Box::new(
                OutputToZip (
                    zip_next::ZipWriter::new(
                        std::fs::File::create(dst).expect("Failed to open output for writing")
                    )
                )
            )
        },
        "tar" => {
            eprintln!("Writing output to {dst:?} as TAR");
            Box::new(OutputToTar (
                tar::Builder::new(
                    Box::new(
                        std::fs::File::create(dst).expect("Failed to open output for writing")
                    )
                )
            ))
        },
        _ => panic!("Unable to determine output format from {dst:?}")
    }
}
