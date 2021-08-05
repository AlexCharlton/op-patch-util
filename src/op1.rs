use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OP1Data {
    Drum {
        name: String,     // "user"
        drum_version: u8, // 1
        octave: u8,       // 0
        start: [u32; 24],
        end: [u32; 24],
        pitch: [i16; 24], // yes -24320/0/24567 512 per semitone ;-1 semitone starts at -256; -48 to +48
        reverse: [u16; 24], // yes 8192/16384
        volume: [u16; 24], // yes 0/8192/16384
        playmode: [u16; 24], // no 0/8192/16384
        dyna_env: [u16; 8], // 0-8182?
        lfo_active: bool,
        lfo_type: LFOType,
        lfo_params: [u16; 8], // 0-16000?
        fx_active: bool,
        fx_type: FXType,
        fx_params: [u16; 8], // 0-16000?
    },
    Sampler {
        name: String,      // "user"
        synth_version: u8, // 2
        octave: u8,        // 0
        base_freq: u16,    // 440
        adsr: [u16; 8],    // 0 - 32767
        knobs: [u16; 8],   // 0 - 32767
        lfo_active: bool,
        lfo_type: LFOType,
        lfo_params: [u16; 8], // 0-16000?
        fx_active: bool,
        fx_type: FXType,
        fx_params: [u16; 8], // 0-16000?
    },
}

impl OP1Data {
    pub fn default_drum() -> Self {
        Self::Drum {
            name: "user".to_string(),
            drum_version: 1,
            octave: 0,
            start: [0; 24],
            end: [0; 24],
            pitch: [0; 24],
            reverse: [8192; 24],
            volume: [8192; 24],
            playmode: [8192; 24],
            dyna_env: [0, 8192, 0, 8192, 0, 0, 0, 0],
            fx_active: false,
            fx_type: FXType::Delay,
            fx_params: [8000; 8],
            lfo_active: false,
            lfo_type: LFOType::Tremolo,
            lfo_params: [16000, 16000, 16000, 16000, 0, 0, 0, 0],
        }
    }

    #[allow(dead_code)]
    pub fn default_sampler() -> Self {
        Self::Sampler {
            name: "user".to_string(),
            synth_version: 2,
            octave: 0,
            base_freq: 440,
            adsr: [64, 10746, 32767, 10000, 4000, 64, 4000, 4000],
            knobs: [0, 0, 22501, 22501, 8192, 0, 6183, 8192],
            fx_active: false,
            fx_type: FXType::Delay,
            fx_params: [8000; 8],
            lfo_active: false,
            lfo_type: LFOType::Tremolo,
            lfo_params: [16000, 16000, 16000, 16000, 0, 0, 0, 0],
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = serde_json::to_vec(self).unwrap();
        if vec.len() % 2 == 1 {
            vec.push(0);
        }
        vec
    }

    pub fn shift_samples(&mut self, n: i8) -> Result<(), String> {
        if n > 23 || n < -23 {
            return Err("Cannot shift beyond 23 semitones".to_string());
        }
        match self {
            Self::Sampler { .. } => return Err("Cannot shift a synth sample".to_string()),
            Self::Drum {
                start,
                end,
                pitch,
                reverse,
                volume,
                playmode,
                ..
            } => {
                if n > 0 {
                    let n = n as usize;
                    start.rotate_right(n);
                    end.rotate_right(n);
                    pitch.rotate_right(n);
                    reverse.rotate_right(n);
                    volume.rotate_right(n);
                    playmode.rotate_right(n);
                } else {
                    let n = n.abs() as usize;
                    start.rotate_left(n);
                    end.rotate_left(n);
                    pitch.rotate_left(n);
                    reverse.rotate_left(n);
                    volume.rotate_left(n);
                    playmode.rotate_left(n);
                }
            }
        }

        Ok(())
    }

    pub fn silence(&mut self, keys: Vec<u8>) -> Result<(), String> {
        match self {
            Self::Sampler { .. } => return Err("Cannot silence a synth sample".to_string()),
            Self::Drum { volume, .. } => {
                for &key in keys.iter() {
                    if key < 1 || key > 24 {
                        return Err(format!("Key {} out of range (1-24)", key));
                    }
                    volume[key as usize - 1] = 0;
                }
            }
        }
        Ok(())
    }
}

impl Default for OP1Data {
    fn default() -> Self {
        Self::default_drum()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
// #[serde(untagged)]
pub enum FXType {
    Delay,
    // Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
// #[serde(untagged)]
pub enum LFOType {
    Tremolo,
    // Other(String),
}
