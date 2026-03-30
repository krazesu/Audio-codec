pub mod wav;
pub mod flac;

fn main() {
    let file_path = String::from("sample_audio/music_a.wav");

    let wave_obj = wav::WaveReader::open_pcm(&file_path);

    if let Err(e) = wave_obj {
        println!("Error while reading file {}: {}", file_path, e);
    }
    else if let Ok(wave_obj_safe) = wave_obj {
        println!("File {} opened successfully!", file_path);
        println!("{}", wave_obj_safe);

        for each_chunk in wave_obj_safe.data_chunks {
            for (i, each_sample) in each_chunk.byte_rate().enumerate() {
                println!("each sample index {} len {}", i, each_sample.len());
            }
        }
    }
}
