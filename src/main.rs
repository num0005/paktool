#![feature(seek_convenience)]
#![feature(seek_stream_len)]
use std::{env, fs::File, io::{self, Read, Seek, SeekFrom, Write}};
use miniz_oxide::{deflate::{compress_to_vec_zlib}};

// start of ever pak uncompressed pak file
const PAK_MAGIC : u64 = 0x6B617052455331;

const HEADER_SIZE: u64 = 0x600000;
const MAX_SECTION_COUNT: u64 = (HEADER_SIZE / 0x8) - 1;
const SECTION_SIZE: u64 = 0x8000;
const MAX_FILE_SIZE: u64 = SECTION_SIZE * MAX_SECTION_COUNT;

fn read_u64(mut file: &File) -> io::Result<u64> {
    let mut buffer: [u8; 8] = [0; 8];
    file.read(&mut buffer)?;
    Ok(u64::from_le_bytes(buffer))
}

fn write_u64(mut file: &File, value: u64) -> io::Result<()> {
    let buffer = u64::to_le_bytes(value);
    file.write_all(&buffer)
}

fn unpack(mut input_file: File, mut output_file: File) -> io::Result<()>
{
    let section_count = read_u64(&input_file)?;

    if (section_count > MAX_SECTION_COUNT) || (section_count == 0) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "ERROR: Bad section count!"))
    }


    #[derive(Clone, Copy, Debug)]
    struct CompressedSection {
        pub size: u64,
        pub offset: u64
    }

    let mut sections = vec![CompressedSection { size: 0, offset: 0 }; section_count as usize];

    for section in 0..(section_count as usize) {
        let offset = read_u64(&input_file)?;
        sections[section].offset = offset;

        if section == 0 {   
            continue;
        } 

        sections[section - 1].size = offset - sections[section - 1].offset;
        if section == (section_count as usize - 1) {
            sections[section].size = input_file.stream_len()? - offset;
        }
    }

    println!("Decompressing {} chunks, this might take a while.", section_count);
    let bar = indicatif::ProgressBar::new(section_count);

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

fn pack(mut input_file: File, mut output_file: File) -> io::Result<()>
{
    let input_size = input_file.stream_len()?;
    if input_size > MAX_FILE_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "ERROR: File too large!"))
    }

    output_file.write(&[0u8; HEADER_SIZE as usize])?;

    let mut section_offsets = vec![];

    let bar = indicatif::ProgressBar::new(input_size);

    println!("Compressing file this might take a while.");

    loop {
        let mut section_data = vec![0u8; SECTION_SIZE as usize];
        let len = input_file.read(&mut section_data[..])?;

        if len == 0 {
            break;
        }

        section_offsets.push(output_file.stream_len()?);

        let compressed_data = compress_to_vec_zlib(&section_data[..len], 1);
        output_file.write_all(&compressed_data)?;

        bar.inc(len as u64);
    }

    bar.finish();

    output_file.seek(SeekFrom::Start(0))?;
    write_u64(&output_file, section_offsets.len() as u64)?;
    for section_offset in section_offsets {
        write_u64(&output_file, section_offset)?;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "ERROR: No pak file supplied"));
    }

    let file_name = &args[1];

    let mut input_file = File::open(file_name)?;

    let pak_magic = read_u64(&input_file)?;
    input_file.seek(SeekFrom::Start(0))?;

    // pack if unpacked
    if pak_magic == PAK_MAGIC {
        let output_file = File::create(file_name.replace("_decompressed.p", ".p"))?;
        pack(input_file, output_file)
    } else {
        let output_file = File::create(file_name.replace(".p", "_decompressed.p"))?;
        unpack(input_file, output_file)
    }

   
}
