pub fn select_input(src: &std::path::PathBuf) -> std::io::BufReader<Box<dyn std::io::Read>> {
    
    let boxed: Box<dyn std::io::Read> = if src == std::path::Path::new("-") {
        eprintln!("Reading blend from STDIN.");
        Box::new(std::io::stdin().lock())
    } else {
        eprintln!("Reading blend from {src:?}.");
        let file = match std::fs::File::open(src) {
            Ok(file) => file,
            Err(err) => panic!("Failed to open blend-file: {err}"),
        };
        
        Box::new(file)
    };
    
    std::io::BufReader::new(boxed)
}
