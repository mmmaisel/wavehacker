/******************************************************************************\
    audiofx-rs
    Copyright (C) 2023 Max Maisel

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
\******************************************************************************/
use crate::analyzer::{loudness::Settings as Lufs, rms::Settings as Rms};
use crate::error::Error;
use crate::frame::FrameIterator;
use crate::progress::Progress;
use hound::{WavReader, WavWriter};

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum Mode {
    /// Analyze peak amplitude
    Amplitude,
    /// Analyze LUFS loudness
    Lufs,
    /// Analyze RMS loudness
    Rms,
}

#[derive(Debug, Clone, clap::Args)]
pub struct Settings {
    /// Algorithm to use
    mode: Mode,
    /// Target loudness in dB. Units depends on mode.
    target: f64,
    /// Analyze multiple channels independently
    #[arg(short)]
    channel_independent: bool,
    /// Do not normalize result to stereo.
    /// You should only use this flag if you have to be strictly
    /// EBU R128 compliant.
    #[arg(short)]
    strict_ebur128: bool,
}

impl Settings {
    pub fn normalize<R, W>(
        &self,
        mut input: WavReader<R>,
        mut output: WavWriter<W>,
    ) -> Result<(), Error>
    where
        R: std::io::Read + std::io::Seek,
        W: std::io::Write + std::io::Seek,
    {
        let spec = input.spec();
        let duration = input.duration();

        let gain = match &self.mode {
            Mode::Amplitude => panic!("Not implemented yet!"),
            Mode::Lufs => {
                let analyzer =
                    Lufs::new(self.channel_independent, self.strict_ebur128);
                let loudness = analyzer.analyze(&mut input)?;
                loudness
                    .iter()
                    .map(|x| {
                        (10.0_f64.powf(self.target / 10.0) / x).sqrt() as f32
                    })
                    .collect::<Vec<f32>>()
            }
            Mode::Rms => {
                let analyzer = Rms::new(self.channel_independent);
                let rms = analyzer.analyze(&mut input)?;
                rms.iter()
                    .map(|x| (10.0_f64.powf(self.target / 20.0) / x) as f32)
                    .collect::<Vec<f32>>()
            }
        };

        input.seek(0)?;
        let mut frames =
            FrameIterator::new(input.samples::<f32>(), spec.channels);
        let mut progress =
            Progress::new(duration as usize, "Processing sample");
        while let Some(frame) = frames.next() {
            progress.next();
            match frame {
                Ok(frame) => {
                    for (i, sample) in frame.iter().enumerate() {
                        let val = if self.channel_independent {
                            sample * gain[i]
                        } else {
                            sample * gain[0]
                        };
                        output.write_sample(val)?;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        output.finalize()?;

        Ok(())
    }
}
