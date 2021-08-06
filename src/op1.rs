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
        pitch: [i16; 24], // -24320/0/24567 512 per semitone ;-1 semitone starts at -256; -48 to +48
        reverse: [u16; 24], // 8192/16384
        volume: [u16; 24], // 0/8192/16384
        playmode: [u16; 24], // 0/8192/16384
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

    pub fn pitch(&mut self, keys: &[u8], pitches: &[f32]) -> Result<(), String> {
        let mut p = 0;
        if pitches.is_empty() {
            return Err("No pitch provided".to_string());
        }

        match self {
            Self::Sampler { .. } => return Err("Cannot pitch a synth sample".to_string()),
            Self::Drum { pitch, .. } => {
                for &key in keys.iter() {
                    if key < 1 || key > 24 {
                        return Err(format!("Key {} out of range (1-24)", key));
                    }
                    let ptch = pitches[p];
                    if ptch < -48.0 || ptch > 48.0 {
                        return Err(format!("Pitch {} out of range (-48-+48)", ptch));
                    }
                    // TODO: Do negative numbers really need special handling?
                    pitch[key as usize - 1] = (512.0 * ptch) as i16;

                    if p + 1 < pitches.len() {
                        p += 1;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn gain(&mut self, keys: &[u8], gains: &[f32]) -> Result<(), String> {
        let mut g = 0;
        if gains.is_empty() {
            return Err("No gains provided".to_string());
        }

        match self {
            Self::Sampler { .. } => return Err("Cannot gain a synth sample".to_string()),
            Self::Drum { volume, .. } => {
                for &key in keys.iter() {
                    if key < 1 || key > 24 {
                        return Err(format!("Key {} out of range (1-24)", key));
                    }
                    let gain = gains[g];
                    if gain < -1.0 || gain > 1.0 {
                        return Err(format!("Gain {} out of range (-1-+1)", gain));
                    }
                    volume[key as usize - 1] = (8192.0 * (gain + 1.0)) as u16;

                    if g + 1 < gains.len() {
                        g += 1;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn reverse(&mut self, keys: &[u8], rev: bool) -> Result<(), String> {
        match self {
            Self::Sampler { .. } => return Err("Cannot reverse a synth sample".to_string()),
            Self::Drum { reverse, .. } => {
                for &key in keys.iter() {
                    if key < 1 || key > 24 {
                        return Err(format!("Key {} out of range (1-24)", key));
                    }
                    reverse[key as usize - 1] = if rev { 16384 } else { 8192 };
                }
            }
        }
        Ok(())
    }

    pub fn copy(&mut self, keys: &[u8], srcs: &[u8]) -> Result<(), String> {
        let mut s = 0;
        if srcs.is_empty() {
            return Err("No source keys provided".to_string());
        }

        match self {
            Self::Sampler { .. } => return Err("Cannot copy a synth sample".to_string()),
            Self::Drum {
                start,
                end,
                pitch,
                reverse,
                volume,
                playmode,
                ..
            } => {
                for &key in keys.iter() {
                    if key < 1 || key > 24 {
                        return Err(format!("Key {} out of range (1-24)", key));
                    }
                    let src = srcs[s];
                    if src < 1 || src > 24 {
                        return Err(format!("Key {} out of range (1-24)", src));
                    }

                    start[key as usize - 1] = start[src as usize - 1];
                    end[key as usize - 1] = end[src as usize - 1];
                    pitch[key as usize - 1] = pitch[src as usize - 1];
                    reverse[key as usize - 1] = reverse[src as usize - 1];
                    volume[key as usize - 1] = volume[src as usize - 1];
                    playmode[key as usize - 1] = playmode[src as usize - 1];

                    if s + 1 < srcs.len() {
                        s += 1;
                    }
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
