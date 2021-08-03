use crate::op1::OP1Data;
use crate::util::*;

use log;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::{error, fmt};

pub type ChunkID = [u8; 4];

pub const FORM: &ChunkID = b"FORM";
pub const AIFF: &ChunkID = b"AIFF";
pub const COMMON: &ChunkID = b"COMM";
pub const SOUND: &ChunkID = b"SSND";
pub const MARKER: &ChunkID = b"MARK";
pub const INSTRUMENT: &ChunkID = b"INST";
pub const MIDI: &ChunkID = b"MIDI";
pub const RECORDING: &ChunkID = b"AESD";
pub const APPLICATION: &ChunkID = b"APPL";
pub const COMMENTS: &ChunkID = b"COMT";
pub const NAME: &ChunkID = b"NAME";
pub const AUTHOR: &ChunkID = b"AUTH";
pub const COPYRIGHT: &ChunkID = b"(c) ";
pub const ANNOTATION: &ChunkID = b"ANNO";

pub const AIFF_C: &ChunkID = b"AIFC";
pub const FORMAT_VER: &ChunkID = b"FVER";

pub const OP_1: &ChunkID = b"op-1";

pub type Buffer<'a> = &'a mut Cursor<Vec<u8>>;

pub fn read_aif(file: &mut (impl Read + Seek)) -> Result<FormChunk, ChunkError> {
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let mut cursor = Cursor::new(buffer);
    FormChunk::parse(&mut cursor)
}

#[derive(Debug)]
pub enum ChunkError {
    InvalidID(ChunkID),
    InvalidFormType(ChunkID),
    InvalidSize(i32, i32), // expected, got,
    InvalidData(String),   // failed to parse something
}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl error::Error for ChunkError {}

pub trait Chunk {
    fn parse(buffer: Buffer) -> Result<Self, ChunkError>
    where
        Self: Sized;

    fn write(&self, _file: &mut (impl Write + Seek)) -> Result<usize, io::Error> {
        //unimplemented!();
        Ok(0)
    }
}

#[derive(Debug)]
pub struct FormChunk {
    pub size: i32,
    pub form_type: ChunkID,
    pub common: CommonChunk,
    pub sound: Option<SoundDataChunk>,
    pub comments: Option<CommentsChunk>,
    pub instrument: Option<InstrumentChunk>,
    pub recording: Option<AudioRecordingChunk>,
    pub markers: Option<MarkerChunk>,
    pub texts: Vec<TextChunk>,
    pub midi: Vec<MIDIDataChunk>,
    pub app: Vec<ApplicationSpecificChunk>,
}

impl Chunk for FormChunk {
    fn parse(buf: Buffer) -> Result<FormChunk, ChunkError> {
        let id = read_chunk_id(buf);
        if &id != FORM {
            return Err(ChunkError::InvalidID(id));
        }

        let size = read_i32_be(buf);
        log::info!("form chunk bytes {}", size);
        let mut form_type = [0; 4];
        buf.read_exact(&mut form_type).unwrap();
        match &form_type {
            AIFF | AIFF_C => (),
            _ => Err(ChunkError::InvalidFormType(form_type))?,
        }

        let mut common: Option<CommonChunk> = None;
        let mut sound: Option<SoundDataChunk> = None;
        let mut comments: Option<CommentsChunk> = None;
        let mut instrument: Option<InstrumentChunk> = None;
        let mut recording: Option<AudioRecordingChunk> = None;
        let mut markers: Option<MarkerChunk> = None;
        let mut texts: Vec<TextChunk> = vec![];
        let mut midi: Vec<MIDIDataChunk> = vec![];
        let mut app: Vec<ApplicationSpecificChunk> = vec![];

        let mut id = [0; 4];
        while let Ok(()) = buf.read_exact(&mut id) {
            match &id {
                COMMON => {
                    common = Some(CommonChunk::parse(buf)?);
                }
                SOUND => {
                    sound = Some(SoundDataChunk::parse(buf)?);
                }
                MARKER => {
                    markers = Some(MarkerChunk::parse(buf)?);
                }
                INSTRUMENT => {
                    instrument = Some(InstrumentChunk::parse(buf)?);
                }
                MIDI => {
                    midi.push(MIDIDataChunk::parse(buf)?);
                }
                RECORDING => {
                    recording = Some(AudioRecordingChunk::parse(buf)?);
                }
                APPLICATION => {
                    app.push(ApplicationSpecificChunk::parse(buf)?);
                }
                COMMENTS => {
                    comments = Some(CommentsChunk::parse(buf)?);
                }
                NAME | AUTHOR | COPYRIGHT | ANNOTATION => {
                    texts.push(TextChunk::parse(buf)?);
                }
                FORMAT_VER => {
                    unimplemented!("FVER chunk detected");
                }
                _ => return Err(ChunkError::InvalidID(id)),
            };
        }

        let mut common = common.unwrap();
        common.num_sample_frames = sound.as_ref().map_or(0, |s| s.sample_frames());

        Ok(FormChunk {
            size,
            form_type,
            common,
            sound,
            comments,
            instrument,
            recording,
            texts,
            markers,
            midi,
            app,
        })
    }

    fn write(&self, file: &mut (impl Write + Seek)) -> Result<usize, io::Error> {
        file.write(FORM)?;
        file.write(&0i32.to_be_bytes())?;
        file.write(&self.form_type)?;
        let mut size = 4; // form_type

        size += self.common.write(file)?;

        for chunk in self.app.iter() {
            size += chunk.write(file)?;
        }
        for chunk in self.midi.iter() {
            size += chunk.write(file)?;
        }
        for chunk in self.texts.iter() {
            size += chunk.write(file)?;
        }

        if let Some(chunk) = &self.comments {
            size += chunk.write(file)?;
        }
        if let Some(chunk) = &self.instrument {
            size += chunk.write(file)?;
        }
        if let Some(chunk) = &self.recording {
            size += chunk.write(file)?;
        }
        if let Some(chunk) = &self.markers {
            size += chunk.write(file)?;
        }
        if let Some(chunk) = &self.sound {
            size += chunk.write(file)?;
        }

        file.seek(SeekFrom::Start(4))?;
        file.write(&(size as i32).to_be_bytes())?;

        Ok(size + 8)
    }
}

#[derive(Debug)]
pub struct CommonChunk {
    pub num_channels: i16,
    pub num_sample_frames: u32,
    pub bit_rate: i16,         // in the spec, this is defined as `sample_size`
    pub sample_rate: [u8; 10], // 80 bit extended floating pt num
}

impl Chunk for CommonChunk {
    fn parse(buf: Buffer) -> Result<CommonChunk, ChunkError> {
        let (_size, num_channels, num_sample_frames, bit_rate) = (
            read_i32_be(buf),
            read_i16_be(buf),
            read_u32_be(buf),
            read_i16_be(buf),
        );

        let mut rate_buf = [0; 10]; // 1 bit sign, 15 bits exponent
        buf.read_exact(&mut rate_buf).unwrap();

        Ok(CommonChunk {
            num_channels,
            num_sample_frames,
            bit_rate,
            sample_rate: rate_buf,
        })
    }

    fn write(&self, file: &mut (impl Write + Seek)) -> Result<usize, io::Error> {
        file.write(COMMON)?;
        file.write(&18i32.to_be_bytes())?;
        file.write(&self.num_channels.to_be_bytes())?;
        file.write(&self.num_sample_frames.to_be_bytes())?;
        file.write(&self.bit_rate.to_be_bytes())?;
        file.write(&self.sample_rate)?;
        Ok(16 + 8)
    }
}

pub struct SoundDataChunk {
    pub size: i32,
    pub offset: u32,
    pub block_size: u32,
    pub sound_data: Vec<u8>,
}

impl SoundDataChunk {
    fn sample_frames(&self) -> u32 {
        self.sound_data.len() as u32 / 2
    }
}

impl fmt::Debug for SoundDataChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SoundDataChunk")
            .field("size", &self.size)
            .field("offset", &self.offset)
            .field("block_size", &self.block_size)
            .finish()
    }
}

impl Chunk for SoundDataChunk {
    fn parse(buf: Buffer) -> Result<SoundDataChunk, ChunkError> {
        let size = read_i32_be(buf);
        let offset = read_u32_be(buf);
        let block_size = read_u32_be(buf);

        let mut sound_data = vec![0u8; size as usize - 8];

        let got_size = buf.read(&mut sound_data).unwrap() as i32;
        if size - 8 != got_size {
            log::warn!(
                "Expected sound chunk of size {}, got {}",
                size - 8,
                got_size
            );
        }

        Ok(SoundDataChunk {
            size: got_size + 8,
            offset,
            block_size,
            sound_data: sound_data[..got_size as usize].to_vec(),
        })
    }

    fn write(&self, file: &mut (impl Write + Seek)) -> Result<usize, io::Error> {
        file.write(SOUND)?;
        file.write(&self.size.to_be_bytes())?;
        file.write(&self.offset.to_be_bytes())?;
        file.write(&self.block_size.to_be_bytes())?;
        file.write(&self.sound_data)?;
        Ok(self.size as usize + 8)
    }
}

type MarkerId = i16;
#[derive(Debug)]
pub struct Marker {
    id: MarkerId,
    position: u32,
    marker_name: String,
}

impl Marker {
    // TODO return result
    pub fn from_reader<R: Read>(r: &mut R) -> Marker {
        let id = read_i16_be(r);
        let position = read_u32_be(r);
        let marker_name = read_pstring(r);

        Marker {
            id,
            position,
            marker_name,
        }
    }
}

#[derive(Debug)]
pub struct MarkerChunk {
    pub size: i32,
    pub num_markers: u16,
    pub markers: Vec<Marker>,
}

impl Chunk for MarkerChunk {
    fn parse(buf: Buffer) -> Result<MarkerChunk, ChunkError> {
        let size = read_i32_be(buf);
        let num_markers = read_u16_be(buf);
        let mut markers = Vec::with_capacity(num_markers as usize);
        // is it worth it to read all markers at once ant create from buf?
        // or does the usage of BufReader make it irrelevant?
        for _ in 0..num_markers {
            markers.push(Marker::from_reader(buf));
        }

        Ok(MarkerChunk {
            size,
            num_markers,
            markers,
        })
    }
}

#[derive(Debug)]
pub enum TextChunkType {
    Name,
    Author,
    Copyright,
    Annotation,
}

#[derive(Debug)]
pub struct TextChunk {
    pub chunk_type: TextChunkType,
    pub size: i32,
    pub text: String,
}

impl Chunk for TextChunk {
    fn parse(buf: Buffer) -> Result<TextChunk, ChunkError> {
        buf.seek(SeekFrom::Current(-4)).unwrap();
        let id = read_chunk_id(buf);
        let chunk_type = match &id {
            NAME => TextChunkType::Name,
            AUTHOR => TextChunkType::Author,
            COPYRIGHT => TextChunkType::Copyright,
            ANNOTATION => TextChunkType::Annotation,
            _ => return Err(ChunkError::InvalidID(id)),
        };

        let size = read_i32_be(buf);
        let mut text_bytes = vec![0; size as usize];
        buf.read_exact(&mut text_bytes).unwrap();
        let text = String::from_utf8(text_bytes).unwrap();

        if size % 2 > 0 {
            // if odd, pad byte present - skip it
            let mut skip = [0; 1];
            buf.read_exact(&mut skip).unwrap()
        }

        Ok(TextChunk {
            chunk_type,
            size,
            text,
        })
    }
}

#[derive(Debug)]
pub struct Loop {
    // 0 no looping / 1 foward loop / 2 forward backward loop - use enum?
    play_mode: i16,
    begin_loop: MarkerId,
    end_loop: MarkerId,
}

impl Loop {
    // TODO return result
    pub fn from_reader(r: &mut impl Read) -> Loop {
        let play_mode = read_i16_be(r);
        let begin_loop = read_i16_be(r);
        let end_loop = read_i16_be(r);

        Loop {
            play_mode,
            begin_loop,
            end_loop,
        }
    }
}

// midi note value range = 0..127 (? not the full range?)
#[derive(Debug)]
pub struct InstrumentChunk {
    size: i32,
    base_note: i8,     // MIDI
    detune: i8,        // -50..50
    low_note: i8,      // MIDI
    high_note: i8,     // MIDI
    low_velocity: i8,  // MIDI
    high_velocity: i8, // MIDI
    gain: i16,         // in db
    sustain_loop: Loop,
    release_loop: Loop,
}

impl Chunk for InstrumentChunk {
    fn parse(buf: Buffer) -> Result<InstrumentChunk, ChunkError> {
        let size = read_i32_be(buf);
        let base_note = read_i8_be(buf);
        let detune = read_i8_be(buf);
        let low_note = read_i8_be(buf);
        let high_note = read_i8_be(buf);
        let low_velocity = read_i8_be(buf);
        let high_velocity = read_i8_be(buf);
        let gain = read_i16_be(buf);

        let sustain_loop = Loop::from_reader(buf);
        let release_loop = Loop::from_reader(buf);

        Ok(InstrumentChunk {
            size,
            base_note,
            detune,
            low_note,
            high_note,
            low_velocity,
            high_velocity,
            gain,
            sustain_loop,
            release_loop,
        })
    }
}

#[derive(Debug)]
pub struct MIDIDataChunk {
    size: i32,
    data: Vec<u8>,
}

impl Chunk for MIDIDataChunk {
    fn parse(buf: Buffer) -> Result<MIDIDataChunk, ChunkError> {
        let size = read_i32_be(buf);

        let mut data = vec![0; size as usize];
        buf.read_exact(&mut data).unwrap();

        Ok(MIDIDataChunk { size, data })
    }
}

#[derive(Debug)]
pub struct AudioRecordingChunk {
    size: i32,
    // AESChannelStatusData
    // specified in "AES Recommended Practice for Digital Audio Engineering"
    data: [u8; 24],
}

impl Chunk for AudioRecordingChunk {
    fn parse(buf: Buffer) -> Result<AudioRecordingChunk, ChunkError> {
        let size = read_i32_be(buf);
        if size != 24 {
            return Err(ChunkError::InvalidSize(24, size));
        }

        let mut data = [0; 24];
        buf.read_exact(&mut data).unwrap();

        Ok(AudioRecordingChunk { size, data })
    }
}

#[derive(Debug)]
pub enum ApplicationSpecificChunk {
    OP1 {
        data: OP1Data,
    },
    UnknownApplication {
        size: i32,
        application_signature: ChunkID,
        data: Vec<u8>,
    },
}

impl Chunk for ApplicationSpecificChunk {
    fn parse(buf: Buffer) -> Result<ApplicationSpecificChunk, ChunkError> {
        let size = read_i32_be(buf);
        let application_signature = read_chunk_id(buf);
        let mut data = vec![0; (size - 4) as usize]; // account for sig size
        buf.read_exact(&mut data).unwrap();

        match &application_signature {
            OP_1 => {
                let end = data.iter().position(|&x| x == 0).unwrap_or(data.len());
                let ds: Result<OP1Data, _> = serde_json::from_slice(&data[0..end]);
                match ds {
                    Ok(data) => Ok(ApplicationSpecificChunk::OP1 { data }),
                    Err(e) => Err(ChunkError::InvalidData(e.to_string())),
                }
            }
            _ => Ok(ApplicationSpecificChunk::UnknownApplication {
                size,
                application_signature,
                data: data.iter().map(|byte| u8::from_be_bytes([*byte])).collect(),
            }),
        }
    }

    fn write(&self, file: &mut (impl Write + Seek)) -> Result<usize, io::Error> {
        file.write(APPLICATION)?;
        Ok(match self {
            Self::OP1 { data } => {
                let data = data.to_bytes();
                let size = data.len() + 4;
                file.write(&(size as i32).to_be_bytes())?;
                file.write(OP_1)?;
                file.write(&data)?;
                size
            }
            Self::UnknownApplication {
                size,
                application_signature,
                data,
            } => {
                file.write(&size.to_be_bytes())?;
                file.write(application_signature)?;
                file.write(&data)?;
                *size as usize
            }
        } + 8)
    }
}

#[derive(Debug)]
pub struct Comment {
    timestamp: u32,
    marker_id: MarkerId,
    count: u16,
    text: String, // padded to an even # of bytes
}

impl Comment {
    // TODO return result
    pub fn from_reader(r: &mut impl Read) -> Comment {
        let timestamp = read_u32_be(r);
        let marker_id = read_i16_be(r);
        let count = read_u16_be(r);

        let mut str_buf = vec![0; count as usize];
        r.read_exact(&mut str_buf).unwrap();
        let text = String::from_utf8(str_buf).unwrap();

        Comment {
            timestamp,
            marker_id,
            count,
            text,
        }
    }
}

#[derive(Debug)]
pub struct CommentsChunk {
    size: i32,
    num_comments: u16,
    comments: Vec<Comment>,
}

impl Chunk for CommentsChunk {
    fn parse(buf: Buffer) -> Result<CommentsChunk, ChunkError> {
        let size = read_i32_be(buf);
        let num_comments = read_u16_be(buf);

        let mut comments = Vec::with_capacity(num_comments as usize);
        for _ in 0..num_comments {
            comments.push(Comment::from_reader(buf))
        }

        Ok(CommentsChunk {
            size,
            num_comments,
            comments,
        })
    }
}
