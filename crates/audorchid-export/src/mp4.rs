//! AAC priming edit list adapter. Muxide writes moov last; changing it moves no
//! mdat positions or chunk offsets. Input is exclusively our just-produced MP4.
use anyhow::{Context, Result, ensure};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

#[derive(Clone)]
struct Atom {
    kind: [u8; 4],
    body: Vec<u8>,
}
fn parse(data: &[u8]) -> Result<Vec<Atom>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() {
        ensure!(data.len() - at >= 8, "truncated MP4 box");
        let len = u32::from_be_bytes(data[at..at + 4].try_into()?) as usize;
        ensure!(len >= 8 && len <= data.len() - at, "invalid MP4 box length");
        out.push(Atom {
            kind: data[at + 4..at + 8].try_into()?,
            body: data[at + 8..at + len].to_vec(),
        });
        at += len;
    }
    Ok(out)
}
fn pack(atoms: &[Atom]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for a in atoms {
        bytes.extend(u32::try_from(a.body.len() + 8)?.to_be_bytes());
        bytes.extend(a.kind);
        bytes.extend(&a.body);
    }
    Ok(bytes)
}
fn get32(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(
        b.get(at..at + 4).context("short MP4 field")?.try_into()?,
    ))
}
fn put32(b: &mut [u8], at: usize, value: u32) -> Result<()> {
    b.get_mut(at..at + 4)
        .context("short MP4 field")?
        .copy_from_slice(&value.to_be_bytes());
    Ok(())
}
fn rewrite_moov(
    body: &[u8],
    sr: u32,
    delay: u32,
    valid: u64,
    packets: u64,
    frame_samples: u32,
    video_duration: u64,
) -> Result<Vec<u8>> {
    let valid32 = u32::try_from(valid)?;
    let video32 = u32::try_from(video_duration)?;
    let mut moov = parse(body)?;
    let mut audio_tracks = 0;
    for atom in &mut moov {
        if &atom.kind == b"mvhd" {
            ensure!(atom.body.first() == Some(&0), "unexpected mvhd version");
            put32(&mut atom.body, 12, sr)?;
            put32(&mut atom.body, 16, video32.max(valid32))?;
        }
        if &atom.kind != b"trak" {
            continue;
        }
        let mut track = parse(&atom.body)?;
        ensure!(
            !track.iter().any(|a| &a.kind == b"edts"),
            "unexpected pre-existing edit list"
        );
        let mdia_index = track
            .iter()
            .position(|a| &a.kind == b"mdia")
            .context("track missing mdia")?;
        let mut mdia = parse(&track[mdia_index].body)?;
        let handler = mdia
            .iter()
            .find(|a| &a.kind == b"hdlr")
            .context("missing handler")?;
        let audio = handler.body.get(8..12) == Some(b"soun");
        for child in &mut track {
            if &child.kind == b"tkhd" {
                ensure!(child.body.first() == Some(&0), "unexpected tkhd version");
                put32(&mut child.body, 20, if audio { valid32 } else { video32 })?;
            }
        }
        if audio {
            audio_tracks += 1;
            for child in &mut mdia {
                if &child.kind == b"mdhd" {
                    ensure!(child.body.first() == Some(&0), "unexpected mdhd version");
                    ensure!(
                        get32(&child.body, 12)? == 90000,
                        "muxide audio timebase changed"
                    );
                    put32(&mut child.body, 12, sr)?;
                    put32(
                        &mut child.body,
                        16,
                        u32::try_from(packets * frame_samples as u64)?,
                    )?;
                }
                if &child.kind == b"minf" {
                    let mut minf = parse(&child.body)?;
                    for stbl in minf.iter_mut().filter(|a| &a.kind == b"stbl") {
                        let mut table = parse(&stbl.body)?;
                        let stts = table
                            .iter_mut()
                            .find(|a| &a.kind == b"stts")
                            .context("missing stts")?;
                        stts.body = [
                            0u32.to_be_bytes(),
                            1u32.to_be_bytes(),
                            u32::try_from(packets)?.to_be_bytes(),
                            frame_samples.to_be_bytes(),
                        ]
                        .concat();
                        stbl.body = pack(&table)?;
                    }
                    child.body = pack(&minf)?;
                }
            }
            track[mdia_index].body = pack(&mdia)?;
            // Movie timescale = sample_rate, allowing an exact edit duration.
            let mut elst = Vec::new();
            elst.extend(0x01000000u32.to_be_bytes());
            elst.extend(1u32.to_be_bytes());
            elst.extend(valid.to_be_bytes());
            elst.extend((delay as i64).to_be_bytes());
            elst.extend(0x00010000u32.to_be_bytes());
            track.push(Atom {
                kind: *b"edts",
                body: pack(&[Atom {
                    kind: *b"elst",
                    body: elst,
                }])?,
            });
        }
        atom.body = pack(&track)?;
    }
    ensure!(audio_tracks == 1, "expected exactly one AAC track");
    pack(&moov)
}
pub fn apply_audio_edit(
    file: &mut File,
    sr: u32,
    delay: u32,
    valid: u64,
    packets: u64,
    frame_samples: u32,
    video_duration: u64,
) -> Result<()> {
    let length = file.metadata()?.len();
    let mut at = 0u64;
    while at < length {
        file.seek(SeekFrom::Start(at))?;
        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;
        let len = u32::from_be_bytes(header[..4].try_into()?) as u64;
        ensure!(len >= 8 && at + len <= length, "invalid top-level MP4 box");
        if &header[4..] == b"moov" {
            ensure!(
                at + len == length,
                "AAC edit requires moov at EOF (fast_start = false)"
            );
            let mut body = vec![0; usize::try_from(len - 8)?];
            file.read_exact(&mut body)?;
            let new_body = rewrite_moov(
                &body,
                sr,
                delay,
                valid,
                packets,
                frame_samples,
                video_duration,
            )?;
            let packed = pack(&[Atom {
                kind: *b"moov",
                body: new_body,
            }])?;
            file.seek(SeekFrom::Start(at))?;
            file.write_all(&packed)?;
            file.set_len(at + packed.len() as u64)?;
            return Ok(());
        }
        at += len;
    }
    anyhow::bail!("MP4 has no moov")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_boxes_are_rejected() {
        assert!(parse(&[0; 7]).is_err());
        assert!(parse(&[0, 0, 0, 3, 0, 0, 0, 0]).is_err());
    }
}
