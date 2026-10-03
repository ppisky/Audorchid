use anyhow::{Result, anyhow, ensure};
use fdk_aac::enc::{AudioObjectType, BitRate, ChannelMode, Encoder, EncoderParams, Transport};

pub struct AacAudio {
    pub packets: Vec<Vec<u8>>,
    pub frame_samples: u32,
    pub delay_samples: u32,
    pub valid_samples: u64,
    pub padding_samples: u64,
}
pub fn encode_aac(pcm: &[f32], sample_rate: u32, bitrate_kbps: u32) -> Result<AacAudio> {
    ensure!(
        pcm.len().is_multiple_of(2) && !pcm.is_empty(),
        "AAC requires nonempty interleaved stereo"
    );
    let encoder = Encoder::new(EncoderParams {
        bit_rate: BitRate::Cbr(bitrate_kbps * 1000),
        sample_rate,
        transport: Transport::Adts,
        channels: ChannelMode::Stereo,
        audio_object_type: AudioObjectType::Mpeg4LowComplexity,
    })
    .map_err(|e| anyhow!("AAC init: {e}"))?;
    let info = encoder.info().map_err(|e| anyhow!("AAC info: {e}"))?;
    let frame_samples = info.frameLength;
    let mut output = vec![0u8; (info.maxOutBufBytes as usize).max(8192)];
    let mut packets = Vec::new();
    // Convert one codec frame at a time, and obey the native consumption count.
    for block in pcm.chunks(frame_samples as usize * 2) {
        let quantized: Vec<i16> = block
            .iter()
            .map(|s| (s * 32768.).round().clamp(-32768., 32767.) as i16)
            .collect();
        let mut offset = 0;
        while offset < quantized.len() {
            let encoded = encoder
                .encode(&quantized[offset..], &mut output)
                .map_err(|e| anyhow!("AAC encode: {e}"))?;
            ensure!(
                encoded.input_consumed > 0 && encoded.input_consumed <= quantized.len() - offset,
                "AAC encoder stopped consuming PCM"
            );
            offset += encoded.input_consumed;
            if encoded.output_size > 0 {
                packets.push(output[..encoded.output_size].to_vec());
            }
        }
    }
    let mut eof = false;
    for _ in 0..64 {
        match encoder.flush(&mut output) {
            Ok(encoded) => {
                if encoded.output_size > 0 {
                    packets.push(output[..encoded.output_size].to_vec());
                }
            }
            Err(e) if e.is_eof() => {
                eof = true;
                break;
            }
            Err(e) => return Err(anyhow!("AAC drain: {e}")),
        }
    }
    ensure!(eof, "AAC drain did not terminate");
    let valid_samples = pcm.len() as u64 / 2;
    let coded_samples = packets.len() as u64 * frame_samples as u64;
    let delay_samples = info.nDelay;
    ensure!(
        coded_samples >= valid_samples + delay_samples as u64,
        "AAC output is missing tail samples"
    );
    let padding_samples = coded_samples - valid_samples - delay_samples as u64;
    Ok(AacAudio {
        packets,
        frame_samples,
        delay_samples,
        valid_samples,
        padding_samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_last_frame_drains_and_describes_delay() {
        let result = encode_aac(&vec![0.; 48001 * 2], 48000, 192).unwrap();
        assert_eq!(result.valid_samples, 48001);
        assert!(result.delay_samples > 0);
        assert_eq!(
            result.packets.len() as u64 * result.frame_samples as u64,
            result.valid_samples + result.delay_samples as u64 + result.padding_samples
        );
        assert!(
            result
                .packets
                .iter()
                .all(|p| p.len() > 7 && p[0] == 0xff && p[1] & 0xf0 == 0xf0)
        );
    }
}
