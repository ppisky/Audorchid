//! Original demo compositions. A MIDI score drives a small, self-generated
//! sample bank; no third-party recordings, soundfonts or runtime synth needed.
use std::f32::consts::{FRAC_PI_4, TAU};

pub const SECONDS: u32 = 16;
const BEAT: f32 = 0.5; // 120 BPM, eight bars of 4/4.
const PPQ: u32 = 480;

#[derive(Clone, Copy)]
struct Note {
    beat: f32,
    length: f32,
    pitch: u8,
    velocity: u8,
    channel: u8,
}

fn score(song: usize) -> Vec<Note> {
    let chords = [
        [62, 66, 69, 73],
        [59, 62, 66, 69],
        [55, 59, 62, 66],
        [57, 61, 64, 69],
    ];
    let roots = [38, 35, 31, 33];
    let melody = [
        [78, 81, 76, 74],
        [78, 74, 71, 73],
        [71, 74, 78, 81],
        [76, 73, 71, 69],
    ];
    let order = if song == 0 {
        [0, 1, 2, 3, 0, 2, 3, 1]
    } else {
        [1, 2, 0, 3, 1, 2, 3, 0]
    };
    let mut notes = Vec::new();
    for (bar, &chord) in order.iter().enumerate() {
        let start = bar as f32 * 4.;
        // Slightly rolled seventh chords, with a soft offbeat reply.
        for (finger, &pitch) in chords[chord].iter().enumerate() {
            for (beat, length, velocity) in [(0., 2.3, 78), (2.75, 0.8, 52)] {
                notes.push(Note {
                    beat: start + beat + finger as f32 * 0.016,
                    length,
                    pitch,
                    velocity,
                    channel: 0,
                });
            }
        }
        for (beat, length, pitch, velocity) in [
            (0., 1.25, roots[chord], 90),
            (1.5, 0.7, roots[chord], 68),
            (2.5, 0.7, roots[chord] + 7, 72),
            (3.5, 0.35, roots[chord] + 12, 58),
        ] {
            notes.push(Note {
                beat: start + beat,
                length,
                pitch,
                velocity,
                channel: 2,
            });
        }
        let rhythm = if song == 0 {
            [0.5, 1.25, 2., 3.]
        } else {
            [0.25, 1., 2.25, 3.]
        };
        for (step, &beat) in rhythm.iter().enumerate() {
            let pitch = melody[chord][if song == 1 { (step + 2) % 4 } else { step }];
            notes.push(Note {
                beat: start + beat,
                length: if step == 3 { 0.85 } else { 0.62 },
                pitch,
                velocity: [88, 70, 79, 74][step],
                channel: 1,
            });
        }
        // A restrained backbeat, swung eighth-note hats, and occasional fills.
        for beat in [0., 2., 3.5] {
            if bar == 0 && beat == 3.5 {
                continue;
            }
            notes.push(Note {
                beat: start + beat,
                length: 0.3,
                pitch: 36,
                velocity: 95,
                channel: 9,
            });
        }
        for beat in [1., 3.] {
            notes.push(Note {
                beat: start + beat + 0.024,
                length: 0.25,
                pitch: 38,
                velocity: if bar == 0 { 55 } else { 76 },
                channel: 9,
            });
        }
        for step in 0..8 {
            let offbeat = step % 2 == 1;
            notes.push(Note {
                beat: start + step as f32 * 0.5 + if offbeat { 0.065 } else { 0. },
                length: 0.18,
                pitch: if step == 7 && bar % 2 == 1 { 46 } else { 42 },
                velocity: if offbeat { 48 } else { 67 },
                channel: 9,
            });
        }
        if bar == 3 || bar == 6 {
            notes.push(Note {
                beat: start + 3.75,
                length: 0.15,
                pitch: 38,
                velocity: 40,
                channel: 9,
            });
        }
    }
    notes
}

struct Sample {
    root: u8,
    pcm: Vec<f32>,
    level: f32,
}

fn noise(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / 8_388_608. - 1.
}

fn sample_bank(sr: u32) -> Vec<Sample> {
    let mut bank = Vec::new();
    for instrument in 0..7 {
        let root = [60, 72, 48, 36, 38, 42, 46][instrument];
        let seconds = [4., 3., 4., 0.65, 0.45, 0.16, 0.55][instrument];
        let frequency = 440. * 2f32.powf((root as f32 - 69.) / 12.);
        let mut state = 0x61c8_8647u32.wrapping_add(instrument as u32);
        let mut previous = 0.;
        let sample_frames = (seconds * sr as f32) as usize;
        let mut pcm = Vec::with_capacity(sample_frames);
        for frame in 0..sample_frames {
            let t = frame as f32 / sr as f32;
            let n = noise(&mut state);
            let high = (n - previous) * 0.5;
            previous = n;
            let phase = TAU * frequency * t;
            let value = match instrument {
                // Warm electric-piano transient with a slowly decaying tine.
                0 => {
                    ((phase + 1.15 * (-3.8 * t).exp() * (phase * 2.).sin()).sin()
                        * (-1.1 * t).exp()
                        + 0.12 * (phase * 3.003).sin() * (-3.2 * t).exp())
                        * (t / 0.006).min(1.)
                }
                // Plucked, pitched sample with a rounded attack and upper partials.
                1 => {
                    ((phase.sin()
                        + 0.34 * (phase * 2.001).sin() * (-2. * t).exp()
                        + 0.14 * (phase * 3.).sin() * (-4. * t).exp()
                        + 0.055 * (phase * 5.).sin() * (-9. * t).exp())
                        * (-2.2 * t).exp()
                        + high * 0.018 * (-90. * t).exp())
                        * (t / 0.003).min(1.)
                }
                2 => {
                    (phase.sin() + 0.28 * (2. * phase).sin() + 0.09 * (3. * phase).sin())
                        * (-1.15 * t).exp()
                        * (t / 0.012).min(1.)
                }
                3 => {
                    (TAU * (48. * t + 2.75 * (1. - (-40. * t).exp()))).sin() * (-10. * t).exp()
                        + high * 0.12 * (-180. * t).exp()
                }
                4 => {
                    high * 0.85 * (-22. * t).exp()
                        + (TAU * 180. * t).sin() * 0.32 * (-30. * t).exp()
                }
                5 | 6 => {
                    (high * 0.7 + 0.1 * (TAU * 7141. * t).sin() + 0.08 * (TAU * 5347. * t).sin())
                        * (if instrument == 5 { -70. * t } else { -12. * t }).exp()
                }
                _ => unreachable!(),
            };
            // End every sample smoothly, including percussive one-shots.
            pcm.push(value * ((seconds - t) / 0.025).clamp(0., 1.));
        }
        bank.push(Sample {
            root,
            pcm,
            level: [0.15, 0.25, 0.28, 0.42, 0.2, 0.07, 0.06][instrument],
        });
    }
    bank
}

pub fn render(song: usize, sr: u32) -> (Vec<[f32; 2]>, Vec<u8>) {
    let notes = score(song);
    let bank = sample_bank(sr);
    let frames = (SECONDS * sr) as usize;
    let mut dry = vec![[0.; 2]; frames];
    let mut send = vec![[0.; 2]; frames];
    for note in &notes {
        let instrument = match note.channel {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => match note.pitch {
                36 => 3,
                38 => 4,
                42 => 5,
                _ => 6,
            },
        };
        let sample = &bank[instrument];
        let ratio = 2f32.powf((note.pitch as f32 - sample.root as f32) / 12.);
        let start = (note.beat * BEAT * sr as f32).round() as usize;
        let gate = note.length * BEAT;
        let release = [0.22, 0.18, 0.08, 0., 0., 0., 0.][instrument];
        let count =
            ((sample.pcm.len() as f32 / ratio).ceil() as usize).min(frames.saturating_sub(start));
        let pan = match instrument {
            0 => (note.pitch as f32 - 65.) * 0.027,
            1 => {
                if song == 0 {
                    0.22
                } else {
                    -0.18
                }
            }
            5 | 6 => 0.38,
            _ => 0.,
        }
        .clamp(-0.65, 0.65);
        let gains = [
            ((pan + 1.) * FRAC_PI_4).cos(),
            ((pan + 1.) * FRAC_PI_4).sin(),
        ];
        for frame in 0..count {
            let position = frame as f32 * ratio;
            let index = position as usize;
            if index >= sample.pcm.len() {
                break;
            }
            let fraction = position.fract();
            let a = sample.pcm[index];
            let b = sample.pcm.get(index + 1).copied().unwrap_or(0.);
            let t = frame as f32 / sr as f32;
            let envelope = if instrument < 3 {
                ((gate + release - t) / release).clamp(0., 1.)
            } else {
                1.
            };
            let value = (a + (b - a) * fraction)
                * envelope
                * sample.level
                * (note.velocity as f32 / 100.).powf(1.4);
            for (channel, &gain) in gains.iter().enumerate() {
                dry[start + frame][channel] += value * gain;
                if instrument < 2 {
                    send[start + frame][channel] += value * gain;
                }
            }
        }
    }
    // Sparse stereo reflections plus a tempo-synchronised, cross-channel echo.
    // Only melodic voices feed the room, so the bass and kick remain clear.
    let taps = [
        (0.031, 0.11),
        (0.053, 0.09),
        (0.089, 0.065),
        (0.137, 0.045),
        (0.211, 0.032),
        (0.375, 0.15),
        (0.75, 0.065),
    ];
    for (seconds, level) in taps {
        let delay = (seconds * sr as f32).round() as usize;
        for frame in delay..frames {
            dry[frame][0] += send[frame - delay][1] * level;
            dry[frame][1] += send[frame - delay][0] * level;
        }
    }
    let peak = dry.iter().flatten().fold(0f32, |peak, x| peak.max(x.abs()));
    let gain = 0.8 / peak.max(0.01);
    for (frame, pair) in dry.iter_mut().enumerate() {
        let t = frame as f32 / sr as f32;
        let fade = (t / 0.012).min(1.) * ((SECONDS as f32 - t) / 0.32).clamp(0., 1.);
        for value in pair {
            *value *= gain * fade;
        }
    }
    (dry, midi(&notes, song))
}

fn variable(mut value: u32, bytes: &mut Vec<u8>) {
    let mut buffer = [0u8; 4];
    let mut index = 3;
    buffer[index] = (value & 0x7f) as u8;
    while {
        value >>= 7;
        value != 0
    } {
        index -= 1;
        buffer[index] = (value & 0x7f) as u8 | 0x80;
    }
    bytes.extend_from_slice(&buffer[index..]);
}
fn chunk(data: &[u8], output: &mut Vec<u8>) {
    output.extend_from_slice(b"MTrk");
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(data);
}
fn midi(notes: &[Note], song: usize) -> Vec<u8> {
    let mut output = b"MThd\0\0\0\x06\0\x01\0\x05\x01\xe0".to_vec();
    let title = if song == 0 {
        "Morning light"
    } else {
        "Moonrise"
    };
    let mut conductor = vec![0, 0xff, 3, title.len() as u8];
    conductor.extend_from_slice(title.as_bytes());
    conductor.extend_from_slice(&[
        0, 0xff, 0x51, 3, 0x07, 0xa1, 0x20, 0, 0xff, 0x58, 4, 4, 2, 24, 8,
    ]);
    variable(32 * PPQ, &mut conductor);
    conductor.extend_from_slice(&[0xff, 0x2f, 0]);
    chunk(&conductor, &mut output);
    for (channel, program, pan) in [(0, 4, 58), (1, 10, 78), (2, 33, 64), (9, 0, 64)] {
        let mut track = vec![0, 0xc0 | channel, program, 0, 0xb0 | channel, 10, pan];
        let mut events = Vec::new();
        for note in notes.iter().filter(|note| note.channel == channel) {
            let start = (note.beat * PPQ as f32).round() as u32;
            let end = ((note.beat + note.length) * PPQ as f32).round() as u32;
            events.push((start, [0x90 | channel, note.pitch, note.velocity]));
            events.push((end, [0x80 | channel, note.pitch, 0]));
        }
        events.sort_by_key(|(tick, message)| (*tick, message[0]));
        let mut previous = 0;
        for (tick, message) in events {
            variable(tick - previous, &mut track);
            track.extend_from_slice(&message);
            previous = tick;
        }
        variable((32 * PPQ).saturating_sub(previous), &mut track);
        track.extend_from_slice(&[0xff, 0x2f, 0]);
        chunk(&track, &mut output);
    }
    output
}
