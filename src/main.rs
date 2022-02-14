use clap::Parser;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
struct Args {
    /// Input file name
    #[clap(short, long)]
    input: PathBuf,

    /// Blend input file
    #[clap(short, long)]
    blend: Option<PathBuf>,

    /// Output file name
    #[clap(short, long)]
    output: PathBuf,
}

fn mean(x: &[i16]) -> i16 {
    (x.iter().map(|&x| x as i32).sum::<i32>() / x.len() as i32) as i16
}

fn read_mono_audio(path: &Path) -> std::io::Result<(Vec<i16>, u32)> {
    let mut input = File::open(path)?;
    let (header, data) = wav::read(&mut input)?;

    // Convert to 16-bit sample, for simplicity
    let data = match data {
        wav::BitDepth::Eight(v) => v.into_iter().map(|x| x as i16).collect(),
        wav::BitDepth::Sixteen(v) => v,
        wav::BitDepth::TwentyFour(v) => v.into_iter().map(|x| (x >> 8) as i16).collect(),
        wav::BitDepth::ThirtyTwoFloat(v) => v.into_iter().map(|x| (x * 32767f32) as i16).collect(),
        wav::BitDepth::Empty => Vec::new(),
    };

    // Convert audio to monotonic
    let data: Vec<_> = data.chunks(header.channel_count as _).map(mean).collect();

    Ok((data, header.sampling_rate))
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let (data, rate) = read_mono_audio(&args.input)?;

    let blend = match &args.blend {
        Some(path) => Some(read_mono_audio(path)?),
        None => None,
    };

    // Generate out-phased audio
    let data: Vec<_> = match blend {
        None => data.into_iter().flat_map(|x| [x, -x]).collect(),
        Some((blend, brate)) => {
            assert!(rate == brate);
            data.into_iter()
                .zip(blend.into_iter().chain(std::iter::repeat(0)))
                .flat_map(|(x, y)| [y/16 - x, y/16 + x])
                .collect()
        }
    };

    let mut output = File::create(&args.output)?;
    wav::write(
        wav::Header::new(wav::header::WAV_FORMAT_PCM, 2, rate, 16),
        &wav::BitDepth::Sixteen(data),
        &mut output,
    )?;

    Ok(())
}
