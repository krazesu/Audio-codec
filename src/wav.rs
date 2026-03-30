use std::fmt;
use std::fs::File;
use std::path::Path;
use std::error;
use std::io::{self, BufReader, Error, Read, Seek, SeekFrom};

use byteorder::{ByteOrder, LittleEndian, BigEndian};

pub struct PCMWaveInfo {
    pub riff_header: RiffChunk,
    pub fmt_header: PCMWaveFormatChunk,
    pub data_chunks: Vec <PCMWaveDataChunk>,
}

pub struct RiffChunk {
    pub file_size: u32,
    pub is_big_endian: bool,
}

#[derive(Clone, Copy)]
pub struct PCMWaveFormatChunk {
    pub num_channels: u16,
    pub samp_rate: u32,
    pub bps: u16,
}

pub struct PCMWaveDataChunk {
    pub size_bytes: u32,
    pub format: PCMWaveFormatChunk,
    pub data_buf: io::BufReader<File>,
}

pub struct PCMWaveDataChunkWindow {
    chunk_size: usize,
    data_chunk: PCMWaveDataChunk,
}

pub struct WaveReader;

#[derive(Debug)]
pub enum WaveReaderError {
    NotRiffError,
    NotWaveError,
    NotPCMError,
    ChunkTypeError,
    DataAlignmentError,
    ReadError,
    // added IoError and FormatError for test suite template to work 
    IoError(io::Error),
    FormatError(String),
}

impl WaveReader {
    pub fn open_pcm(file_path: &str) -> Result <PCMWaveInfo, WaveReaderError> {
        let mut wav_file = match File::open(file_path)  {
            Ok(file)    => file,
            Err(e)      => return Err(WaveReaderError::ReadError),
        };

        // riff_header: RiffChunk
        let riff_header = WaveReader::read_riff_chunk(&mut wav_file)?;
        
        //fmt_header: PCMWaveFormatChunk
        wav_file.seek(SeekFrom::Start(12));
        let fmt_header = WaveReader::read_fmt_chunk(&mut wav_file)?;

        //data_chunks: Vec <PCMWaveDataChunk>   
        let mut data_chunks: Vec<PCMWaveDataChunk> = Vec::new();
        let mut current_pos = 12 + 24; // Skip RIFF and fmt chunks

        //todo!(populate vec with data in the data chunks)
        while current_pos < riff_header.file_size as u64 + 8 {
            wav_file.seek(SeekFrom::Start(current_pos))?;
            let mut header = [0u8; 8];
            wav_file.read_exact(&mut header)?;
            let chunk_id = &header[0..4];
            let chunk_size = LittleEndian::read_u32(&header[4..8]);

            if chunk_id == b"data" {
                let data_chunk = WaveReader::read_data_chunk(current_pos + 8, &fmt_header, wav_file.try_clone().unwrap())?;
                data_chunks.push(data_chunk);
            }

            current_pos += 8 + chunk_size as u64;
        }

        Ok(PCMWaveInfo {
            riff_header,
            fmt_header,
            data_chunks,
        })
    }

    fn read_riff_chunk(fh: &mut File) -> Result <RiffChunk, WaveReaderError> {
        let wav_file = fh;

        wav_file.seek(SeekFrom::Start(0));

        let mut riff_chunk_buffer = [0u8; 12];

        wav_file.read_exact(&mut riff_chunk_buffer);

        let chunk_id = &riff_chunk_buffer[0..=3];    // chunk_id = bytes 0 to 3 (big) "RIFF"
        let chunk_id_ascii:String = chunk_id.iter().map(|&x| x as char).collect();

        //todo!(error handling: NotRiffError if chunk_id != "RIFF" or chunk_id != "RIFX");
        if chunk_id_ascii != "RIFF" && chunk_id_ascii != "RIFX" {
            return Err(WaveReaderError::NotRiffError);
        }

        //Format
        let format = &riff_chunk_buffer[8..=11];    // format = bytes 8 to 11 (big) "WAVE"
        let format_ascii:String = format.iter().map(|&x| x as char).collect();

        //todo!(error handling: NotWaveError if format != "WAVE");
        if format_ascii != "WAVE"   {
            return Err(WaveReaderError::NotWaveError);
        }
        
        // is_big_endian: bool
        let mut is_big_endian = false;

        // check for RIFX (endianness)
        if chunk_id_ascii == "RIFX" { 
            is_big_endian = true;
        }

        // file_size: u32
        let file_size_vec = &riff_chunk_buffer[4..=7];  // file_size = bytes 4 to 7 (little)

        if is_big_endian   {
            let file_size = BigEndian::read_u32(&file_size_vec);

            Ok(RiffChunk {
                file_size,
                is_big_endian,
            })
        }
        else {
            let file_size = LittleEndian::read_u32(&file_size_vec);    // convert file_size_vec to u32

            Ok(RiffChunk {
                file_size,
                is_big_endian,
            })
        }

    }

    fn read_fmt_chunk(fh: &mut File) -> Result <PCMWaveFormatChunk, WaveReaderError> {
        let wav_file = fh;

        let mut fmt_subchunk_buffer = [0u8; 24];

        wav_file.read_exact(&mut fmt_subchunk_buffer);

        let subchunk1_id = &fmt_subchunk_buffer[0..=3]; // subchunk1_id = bytes 0 to 3 (big) "fmt "
        let subchunk1_id_ascii:String = subchunk1_id.iter().map(|&x| x as char).collect();

        if subchunk1_id_ascii != "fmt " {
            return Err(WaveReaderError::ChunkTypeError);
        }

        // num_channels: u16
        let num_channels_vec = &fmt_subchunk_buffer[10..=11];   // num_channels = bytes 10 to 11
        let num_channels = LittleEndian::read_u16(&num_channels_vec);   // convert to u16 little endian

        // samp_rate: u32
        let samp_rate_vec = &fmt_subchunk_buffer[12..=15];  // samp_rate = bytes 12 to 15
        let samp_rate = LittleEndian::read_u32(&samp_rate_vec); // convert to u32 little endian

        // bps: u16
        let bps_vec = &fmt_subchunk_buffer[22..=23];    // bps = bytes 16 to 19
        let bps = LittleEndian::read_u16(&bps_vec); // convert to u16 little endian

        Ok(PCMWaveFormatChunk {
            num_channels,
            samp_rate,
            bps,
        })
    }

    fn read_data_chunk(start_pos: u64, fmt_info: &PCMWaveFormatChunk, fh: File) -> Result <PCMWaveDataChunk, WaveReaderError> {
        let mut wav_file = fh;

        wav_file.seek(SeekFrom::Start(start_pos));

        let mut data_bytes = Vec::new();
        wav_file.read_to_end(&mut data_bytes)?;

        // size_bytes: u32 (# of samples in the data chunk)
        let mut size_bytes = [data_bytes[0], data_bytes[1], data_bytes[2], data_bytes[3]];
        let size_bytes = u32::from_le_bytes(size_bytes);

        // format: PCMWaveFormatChunk
        let format = PCMWaveFormatChunk {
            num_channels: fmt_info.num_channels,
            samp_rate: fmt_info.samp_rate,
            bps: fmt_info.bps,
        };

        if (data_bytes.len() - 4 != size_bytes as usize) {
            return Err(WaveReaderError::IoError(io::Error::new(io::ErrorKind::Other, "failed to fill whole buffer")));
        }
        

        // data_buf: io::BufReader<File>
        let data_buf = io::BufReader::with_capacity(fmt_info.block_align() as usize, wav_file);

        Ok(PCMWaveDataChunk {
            size_bytes,
            format,
            data_buf,
        })
    }
}

impl PCMWaveFormatChunk {
    fn byte_rate(&self) -> u32 {
        return (self.samp_rate * self.num_channels as u32 * (self.bps as u32 / 8)) as u32;
    }

    fn block_align(&self) -> u16 {
        return (self.num_channels * (self.bps / 8)) as u16;
    }
}

impl PCMWaveDataChunk {
    pub fn chunks_byte_rate(self) -> PCMWaveDataChunkWindow {
        let chunk_size = self.format.byte_rate() as usize;
        PCMWaveDataChunkWindow {
            chunk_size,
            data_chunk: self,
        }
    }

    pub fn chunks(self, chunk_size: usize) -> PCMWaveDataChunkWindow {
        PCMWaveDataChunkWindow {
            chunk_size,
            data_chunk: self,
        }
    }
}

impl Iterator for PCMWaveDataChunkWindow {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer = vec![0u8; self.chunk_size];
        match self.data_chunk.data_buf.read_exact(&mut buffer) {
            Ok(_) => Some(buffer),
            Err(_) => None,
        }
    }
}

impl fmt::Display for WaveReaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaveReaderError::NotRiffError       => write!(f, "NotRiffError"),
            WaveReaderError::NotWaveError       => write!(f, "NotWaveError"),
            WaveReaderError::NotPCMError        => write!(f, "NotPCMError"),
            WaveReaderError::ChunkTypeError     => write!(f, "ChunkTypeError"),
            WaveReaderError::DataAlignmentError => write!(f, "DataAlignmentError"),
            WaveReaderError::ReadError          => write!(f, "ReadError"),
            // added IoError and FormatError cases:
            WaveReaderError::IoError(e)  => write!(f, "IO error: {}", e),           
            WaveReaderError::FormatError(msg) => write!(f, "Format error: {}", msg),
        }
    }
}

// implemented error::Error for WaveReaderError
impl error::Error for WaveReaderError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            WaveReaderError::IoError(e) => Some(e),
            WaveReaderError::FormatError(_) => None,
            _ => None,
        }
    }
}

impl From<io::Error> for WaveReaderError {
    fn from(error: io::Error) -> Self {
        WaveReaderError::IoError(error)
    }
}

impl fmt::Display for PCMWaveInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "riff_header:\n")?;
        write!(f, "       file_size: {}\n", self.riff_header.file_size)?;
        write!(f, "       is_big_endian: {}\n", self.riff_header.is_big_endian)?;

        write!(f, "format_header:\n")?;
        write!(f, "       num_channels: {}\n", self.fmt_header.num_channels)?;
        write!(f, "       samp_rate: {}\n", self.fmt_header.samp_rate)?;
        write!(f, "       bps: {}\n", self.fmt_header.bps)?;
        
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod read_riff {
        use super::*;
        use std::io::Write;

        fn create_temp_file(file_name: &str, content: &[u8]) -> Result <(), io::Error> {
            let mut file = File::create(file_name)?;
            file.write_all(content)?;

            Ok(())
        }
        
        macro_rules! internal_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() -> Result <(), WaveReaderError> {
                    let (input, (will_panic, expected)) = $value;

                    let file_name = format!("midp_{}.wav.part", stringify!($name));
                    let result;
                    {
                        create_temp_file(&file_name, input)?;
                        let mut input_fh = File::open(&file_name)?;
                        result = WaveReader::read_riff_chunk(&mut input_fh);
                    }
                    std::fs::remove_file(&file_name)?;

                    if will_panic {
                        assert!(result.is_err());
                    }
                    else if let Ok(safe_result) = result {
                        assert_eq!(expected.file_size, safe_result.file_size);
                        assert_eq!(expected.is_big_endian, safe_result.is_big_endian);
                    }
                    else {
                        result?;
                    }

                    Ok(())
                }
            )*
            }
        }
        
        internal_tests! {
            it_valid_le_00: (
                &[0x52, 0x49, 0x46, 0x46, 0x0, 0x0, 0x0, 0x0, 0x57, 0x41, 0x56, 0x45],
                (
                    false,
                    RiffChunk {
                        file_size: 0,
                        is_big_endian: false,
                    },
                )),
            it_valid_le_01: (             //little endian
                &[0x52, 0x49, 0x46, 0x46, 0x80, 0x0, 0x0, 0x0, 0x57, 0x41, 0x56, 0x45],
                (
                    false,
                    RiffChunk {
                        file_size: 128,
                        is_big_endian: false,
                    },
                )),
            it_valid_le_02: (              
                &[0x52, 0x49, 0x46, 0x46, 0x1C, 0x40, 0x36, 0x0, 0x57, 0x41, 0x56, 0x45],
                (
                    false,
                    RiffChunk {
                        file_size: 3_555_356,
                        is_big_endian: false,
                    },
                )),
            it_valid_be_00: (                        //big endian
                &[0x52, 0x49, 0x46, 0x58, 0x0, 0x0, 0x0, 0x0, 0x57, 0x41, 0x56, 0x45],
                (
                    false,
                    RiffChunk {
                        file_size: 0,
                        is_big_endian: true,
                    },
                )),
            it_valid_be_01: (
                &[0x52, 0x49, 0x46, 0x58, 0x00, 0x0, 0x0, 0x80, 0x57, 0x41, 0x56, 0x45],
                (
                    false,
                    RiffChunk {
                        file_size: 128,
                        is_big_endian: true,
                    },
                )),
            it_valid_be_02: (
                &[0x52, 0x49, 0x46, 0x58, 0x00, 0x36, 0x40, 0x1C, 0x57, 0x41, 0x56, 0x45],
                (
                    false,
                    RiffChunk {
                        file_size: 3_555_356,
                        is_big_endian: true,
                    },
                )),
            it_bad_riff: (
                &[0x00, 0x49, 0x46, 0x46, 0x00, 0x36, 0x40, 0x1C, 0x57, 0x41, 0x56, 0x45],
                (
                    true,
                    RiffChunk {
                        file_size: 0,
                        is_big_endian: false,
                    },
                )),
            it_bad_wave: (
                &[0x52, 0x49, 0x46, 0x46, 0x00, 0x36, 0x40, 0x1C, 0x57, 0x41, 0x56, 0x00],
                (
                    true,
                    RiffChunk {
                        file_size: 0,
                        is_big_endian: false,
                    },
                )),
        }
    }

    #[cfg(test)]
    mod read_wav_fmt {
        use super::*;
        use std::io::Write;

        fn create_temp_file(file_name: &str, content: &[u8]) -> Result <(), io::Error> {
            let mut file = File::create(file_name)?;
            file.write_all(content)?;

            Ok(())
        }
        
        macro_rules! internal_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() -> Result <(), WaveReaderError> {
                    let (input, (will_panic, expected)) = $value;

                    let file_name = format!("midp_{}.wav.part", stringify!($name));
                    let result;
                    {
                        create_temp_file(&file_name, input)?;
                        let mut input_fh = File::open(&file_name)?;
                        result = WaveReader::read_fmt_chunk(&mut input_fh);
                    }
                    std::fs::remove_file(&file_name)?;

                    if will_panic {
                        assert!(result.is_err());
                    }
                    else if let Ok(safe_result) = result {
                        assert_eq!(expected.num_channels, safe_result.num_channels);
                        assert_eq!(expected.samp_rate, safe_result.samp_rate);
                        assert_eq!(expected.bps, safe_result.bps);
                    }
                    else {
                        result?;
                    }

                    Ok(())
                }
            )*
            }
        }
        
        internal_tests! {
            it_valid_00: (
                &[
                    0x66, 0x6d, 0x74, 0x20,
                    0x10, 0x0, 0x0, 0x0,
                    0x01, 0x0,
                    0x01, 0x0,
                    0x44, 0xac, 0x0, 0x0,
                    0x44, 0xac, 0x0, 0x0,
                    0x01, 0x00, 0x08, 0x0,
                ],
                (
                    false,
                    PCMWaveFormatChunk {
                        num_channels: 1,
                        samp_rate: 44100,
                        bps: 8,
                    },
                )),
            it_valid_01: (
                &[
                    0x66, 0x6d, 0x74, 0x20,
                    0x10, 0x0, 0x0, 0x0,
                    0x01, 0x0,
                    0x02, 0x0,
                    0x44, 0xac, 0x0, 0x0,
                    0x88, 0x58, 0x01, 0x0,
                    0x02, 0x00, 0x08, 0x0,
                ],
                (
                    false,
                    PCMWaveFormatChunk {
                        num_channels: 2,
                        samp_rate: 44100,
                        bps: 8,
                    },
                )),
            it_valid_02: (
                &[
                    0x66, 0x6d, 0x74, 0x20,
                    0x10, 0x0, 0x0, 0x0,
                    0x01, 0x0,
                    0x02, 0x0,
                    0x44, 0xac, 0x0, 0x0,
                    0x10, 0xb1, 0x02, 0x0,
                    0x04, 0x00, 0x10, 0x0,
                ],
                (
                    false,
                    PCMWaveFormatChunk {
                        num_channels: 2,
                        samp_rate: 44100,
                        bps: 16,
                    },
                )),
        }
    }

// checks if the actual number of bytes read using read_data_chunk matches the expected size
    mod read_data {
        use super::*;
        use std::io::Write;
    
        fn create_temp_file(file_name: &str, content: &[u8]) -> Result<(), io::Error> {
            let mut file = File::create(file_name)?;
            file.write_all(content)?;
            Ok(())
        }
    
        macro_rules! internal_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() -> Result<(), WaveReaderError> {
                    let (input, fmt_info, (will_panic, expected)) = $value;
    
                    let file_name = format!("midf_{}.wav.part", stringify!($name));
                    let result;
                    {
                        create_temp_file(&file_name, input)?;
                        let input_fh = File::open(&file_name)?;
                        result = WaveReader::read_data_chunk(40, &fmt_info, input_fh);
                    }
                    let _ = std::fs::remove_file(file_name);
    
                    match (result, will_panic, expected) {
                    // If the test case expects a panic, check if the operation resulted in an error
                         (Err(e), true, Some(exp)) => assert_eq!(format!("{}", e), exp),
                    // If the test case expects a successful result, compare the size of the data chunk, eto mainly nagchecheck ng 
                    // data chunk
                         (Ok(res), false, _) => assert_eq!(res.size_bytes, expected.unwrap().parse::<u32>().unwrap()),
                    // If the test case fails to match the expected outcome, panic
                        _ => panic!("Test failed")
                    }
    
                    Ok(())
                }
            )*
            }
        }
    
    // tests with varying wave formats (e.g. num_channel is 1 or 2, bps is 8 or 16)
        internal_tests! {
            // Test case with a valid data chunk, 2 channels, 16 bps, and 44100 sample rate
            test_pass_data_chunk_44100_16bps: (
                &[
                    // WAV header (12 bytes) + Format chunk (24 bytes) + Data chunk header (8 bytes)
                    b'R', b'I', b'F', b'F', 0x24, 0x00, 0x00, 0x00, b'W', b'A', b'V', b'E',
                    b'f', b'm', b't', b' ', 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x44, 0xac, 0x00, 0x00, 0x10, 0xb1, 0x02, 0x00, 0x04, 0x00, 0x10, 0x00,
                    // Data bytes
                    b'd', b'a', b't', b'a', 0x04, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04
                ],
                // Format information
                PCMWaveFormatChunk {
                    num_channels: 2,
                    samp_rate: 44100,
                    bps: 16,
                },
                // Expected outcome: The size of the data chunk is "4" bytes
                (false, Some("4")),
            ),
            // Test case with a valid data chunk, 1 channel, 8 bps, and 22050 sample rate
            test_pass_data_chunk_22050_8bps: (
                &[
                    // WAV header (12 bytes) + Format chunk (24 bytes) + Data chunk header (8 bytes)
                    b'R', b'I', b'F', b'F', 0x24, 0x00, 0x00, 0x00, b'W', b'A', b'V', b'E',
                    b'f', b'm', b't', b' ', 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x44, 0xac, 0x00, 0x00, 0x44, 0x55, 0x00, 0x00, 0x04, 0x00, 0x08, 0x00,
                    // Data bytes
                    b'd', b'a', b't', b'a', 0x04, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04
                ],
                // Format information
                PCMWaveFormatChunk {
                    num_channels: 1,
                    samp_rate: 22050,
                    bps: 8,
                },
                // Expected outcome: The size of the data chunk is "4" bytes
                (false, Some("4")),
            ),
            // Test case with an invalid data size
            test_fail_invalid_data_size: (
                &[
                    // WAV header (12 bytes) + Format chunk (24 bytes) + Data chunk header (8 bytes)
                    b'R', b'I', b'F', b'F', 0x24, 0x00, 0x00, 0x00, b'W', b'A', b'V', b'E',
                    b'f', b'm', b't', b' ', 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x44, 0xac, 0x00, 0x00, 0x10, 0xb1, 0x02, 0x00, 0x04, 0x00, 0x10, 0x00,
                    // Data chunk header with incorrect data size (less than expected)
                    b'd', b'a', b't', b'a', 0x04, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03
                ],
                // Format information
                PCMWaveFormatChunk {
                    num_channels: 2,
                    samp_rate: 44100,
                    bps: 16,
                },
                // Expected outcome: An IO error is expected due to failed data buffer filling
                (true, Some("IO error: failed to fill whole buffer")),
            ),
        }
    }
}