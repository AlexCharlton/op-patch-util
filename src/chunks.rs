use crate::util::*;
use std::io::BufReader;
use std::io::{Read, Seek};

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

pub fn read_aif(file: impl Read + Seek) -> Result<FormChunk, ChunkError> {
    FormChunk::parse(&mut BufReader::new(file))
}

#[derive(Debug)]
pub enum ChunkError {
    InvalidID(ChunkID),
    InvalidFormType(ChunkID),
    InvalidSize(i32, i32), // expected, got,
                           // InvalidData(&'static str), // failed to parse something
}

pub trait Chunk {
    fn parse(buffer: &mut BufReader<impl Read + Seek>) -> Result<Self, ChunkError>
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct FormChunk {
    size: i32,
    common: CommonChunk,
    sound: Option<SoundDataChunk>,
    comments: Option<CommentsChunk>,
    instrument: Option<InstrumentChunk>,
    recording: Option<AudioRecordingChunk>,
    markers: Option<MarkerChunk>,
    texts: Vec<TextChunk>,
    midi: Vec<MIDIDataChunk>,
    app: Vec<ApplicationSpecificChunk>,
}

impl Chunk for FormChunk {
    fn parse(buf: &mut BufReader<impl Read + Seek>) -> Result<FormChunk, ChunkError> {
        let id = read_chunk_id(buf);
        if &id != FORM {
            return Err(ChunkError::InvalidID(id));
        }

        let size = read_i32_be(buf);
        println!("form chunk bytes {}", size);
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
                    common = Some(CommonChunk::parse(buf).unwrap());
                }
                SOUND => {
                    sound = Some(SoundDataChunk::parse(buf).unwrap());
                }
                MARKER => {
                    markers = Some(MarkerChunk::parse(buf).unwrap());
                }
                INSTRUMENT => {
                    instrument = Some(InstrumentChunk::parse(buf).unwrap());
                }
                MIDI => {
                    midi.push(MIDIDataChunk::parse(buf).unwrap());
                }
                RECORDING => {
                    recording = Some(AudioRecordingChunk::parse(buf).unwrap());
                }
                APPLICATION => {
                    app.push(ApplicationSpecificChunk::parse(buf).unwrap());
                }
                COMMENTS => {
                    comments = Some(CommentsChunk::parse(buf).unwrap());
                }
                NAME | AUTHOR | COPYRIGHT | ANNOTATION => {
                    texts.push(TextChunk::parse(buf).unwrap());
                }
                FORMAT_VER => {
                    unimplemented!("FVER chunk detected");
                }
                _ => return Err(ChunkError::InvalidID(id)),
            };
        }

        Ok(FormChunk {
            size,
            common: common.unwrap(),
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
}

#[derive(Debug)]
pub struct CommonChunk {
    pub size: i32,
    pub num_channels: i16,
    pub num_sample_frames: u32,
    pub bit_rate: i16,         // in the spec, this is defined as `sample_size`
    pub sample_rate: [u8; 10], // 80 bit extended floating pt num
}

impl Chunk for CommonChunk {
    fn parse(buf: &mut BufReader<impl Read>) -> Result<CommonChunk, ChunkError> {
        let (size, num_channels, num_sample_frames, bit_rate) = (
            read_i32_be(buf),
            read_i16_be(buf),
            read_u32_be(buf),
            read_i16_be(buf),
        );

        let mut rate_buf = [0; 10]; // 1 bit sign, 15 bits exponent
        buf.read_exact(&mut rate_buf).unwrap();

        Ok(CommonChunk {
            size,
            num_channels,
            num_sample_frames,
            bit_rate,
            sample_rate: rate_buf,
        })
    }
}

pub struct SoundDataChunk {
    pub size: i32,
    pub offset: u32,
    pub block_size: u32,
    pub sound_data: Vec<u8>,
}

impl std::fmt::Debug for SoundDataChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoundDataChunk")
            .field("size", &self.size)
            .field("offset", &self.offset)
            .field("block_size", &self.block_size)
            .finish()
    }
}

impl Chunk for SoundDataChunk {
    fn parse(buf: &mut BufReader<impl Read>) -> Result<SoundDataChunk, ChunkError> {
        let size = read_i32_be(buf);
        let offset = read_u32_be(buf);
        let block_size = read_u32_be(buf);

        // TODO some sort of streaming read optimization?
        let sound_size = size - 8; // account for offset + block size bytes
        let mut sound_data = vec![0u8; sound_size as usize];

        // buf.read_exact(&mut sound_data).unwrap();
        let got_size = buf.read(&mut sound_data).unwrap();
        dbg!(size, offset, block_size, got_size);

        Ok(SoundDataChunk {
            size,
            offset,
            block_size,
            sound_data,
        })
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
    fn parse(buf: &mut BufReader<impl Read>) -> Result<MarkerChunk, ChunkError> {
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
    fn parse(buf: &mut BufReader<impl Read + Seek>) -> Result<TextChunk, ChunkError> {
        buf.seek_relative(-4).unwrap();
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
    fn parse(buf: &mut BufReader<impl Read>) -> Result<InstrumentChunk, ChunkError> {
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
    fn parse(buf: &mut BufReader<impl Read>) -> Result<MIDIDataChunk, ChunkError> {
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
    fn parse(buf: &mut BufReader<impl Read>) -> Result<AudioRecordingChunk, ChunkError> {
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
pub struct ApplicationSpecificChunk {
    size: i32,
    application_signature: ChunkID, // TODO check if bytes should be i8
    data: Vec<i8>,
}

impl Chunk for ApplicationSpecificChunk {
    fn parse(buf: &mut BufReader<impl Read>) -> Result<ApplicationSpecificChunk, ChunkError> {
        let size = read_i32_be(buf);
        let application_signature = read_chunk_id(buf); // TODO verify
        let mut data = vec![0; (size - 4) as usize]; // account for sig size
        buf.read_exact(&mut data).unwrap();

        Ok(ApplicationSpecificChunk {
            size,
            application_signature,
            data: data.iter().map(|byte| i8::from_be_bytes([*byte])).collect(),
        })
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
    fn parse(buf: &mut BufReader<impl Read>) -> Result<CommentsChunk, ChunkError> {
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
