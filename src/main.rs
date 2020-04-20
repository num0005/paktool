#![feature(seek_convenience)]
use std::{env, fs::File, io::{self, Read, Seek, SeekFrom, Write}};

fn read_u64(mut file: &File) -> io::Result<u64> {
    let mut buffer: Vec<u8> = vec![0; 8];
    file.read(&mut buffer)?;
    Ok(    
        ((buffer[0] as u64) <<  0) |
        ((buffer[1] as u64) <<  8) |
        ((buffer[2] as u64) << 16) |
        ((buffer[3] as u64) << 24) |
        ((buffer[4] as u64) << 32) |
        ((buffer[5] as u64) << 40) |
        ((buffer[6] as u64) << 48) |
        ((buffer[7] as u64) << 56)
    )
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "ERROR: No pak file supplied"));
    }

    let mut input_file = File::open(args[1].clone())?;
    let mut output_file = File::create(args[1].replace(".pak", "_decompressed.pak"))?;

    const MAX_SECTION_COUNT: usize = 0x600000 / 0x8;

    let section_count = read_u64(&input_file)? as usize;

    if (section_count > MAX_SECTION_COUNT) || (section_count == 0) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "ERROR: Bad section count!"))
    }


    #[derive(Clone, Copy, Debug)]
    struct CompressedSection {
        pub size: u64,
        pub offset: u64
    }

    let mut sections = vec![CompressedSection { size: 0, offset: 0 }; section_count];

    for section in 0..section_count {
        let offset = read_u64(&input_file)?;
        sections[section].offset = offset;
        if section == 0 {   
            continue;
        } else if section == (section_count - 1) {
            sections[section].size = input_file.stream_len()? - offset;
        } else {
            sections[section - 1].size = offset - sections[section - 1].offset;
        }
    }

    println!("Decompressing {} chunks, this might take a while.", section_count);
    let bar = indicatif::ProgressBar::new(section_count as u64);

    for section in sections {
        let mut section_data = vec![0u8; section.size as usize];
        input_file.seek(SeekFrom::Start(section.offset as u64))?;
        input_file.read_exact(&mut section_data[..])?;
        match inflate::inflate_bytes_zlib(&section_data[..]) {
            Ok(inflated_data) => output_file.write_all(&inflated_data[..])?,
            Err(error_message) => return Err(io::Error::new(io::ErrorKind::InvalidData, error_message))
        }

        bar.inc(1);
    }

    bar.finish();

    Ok(())
}