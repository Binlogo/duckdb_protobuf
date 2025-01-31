use std::error::Error;
use std::fs::File;
use std::io;

use byteorder::{BigEndian, ReadBytesExt};
use ouroboros::self_referencing;
use protobuf::CodedInputStream;
use strum::{AsRefStr, EnumIter, EnumString, IntoEnumIterator};

use crate::vtab::Parameters;

pub struct RecordsReader {
    files_iterator: glob::Paths,
    length_kind: LengthKind,
    current_file: Option<LengthDelimitedRecordsReader>,
}

impl RecordsReader {
    pub fn new(params: &Parameters) -> Result<RecordsReader, Box<dyn Error>> {
        Ok(RecordsReader {
            files_iterator: glob::glob(params.files.as_str())?,
            length_kind: params.length_kind,
            current_file: None,
        })
    }

    pub fn next_message(&mut self) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let file_reader = if let Some(reader) = &mut self.current_file {
            reader
        } else {
            let Some(next_file_path) = self.files_iterator.next() else {
                return Ok(None);
            };

            let next_file_path = next_file_path?;
            let next_file = File::open(&next_file_path)?;
            self.current_file = Some(LengthDelimitedRecordsReader::create(
                next_file,
                self.length_kind,
            ));

            self.current_file.as_mut().unwrap()
        };

        let Some(next_message) = file_reader.try_get_next()? else {
            self.current_file = None;
            return Ok(None);
        };

        Ok(Some(next_message))
    }
}

#[derive(Copy, Clone, EnumString, EnumIter, AsRefStr)]
pub enum LengthKind {
    BigEndianFixed,
    Varint,
}

pub fn parse<T: std::str::FromStr<Err = impl Error> + IntoEnumIterator + AsRef<str>>(
    value: &str,
) -> Result<T, Box<dyn Error>> {
    Ok(T::from_str(value).map_err(|err| {
        format!(
            "{}: expected one of: {}, got: {}",
            err,
            LengthKind::iter()
                .map(|it| format!("{}", it.as_ref()))
                .collect::<Vec<_>>()
                .join(", "),
            value
        )
    })?)
}

#[self_referencing]
pub struct LengthDelimitedRecordsReader {
    length_kind: LengthKind,
    inner: File,

    #[borrows(mut inner)]
    #[not_covariant]
    reader: CodedInputStream<'this>,
}

impl LengthDelimitedRecordsReader {
    pub fn create(inner: File, length_kind: LengthKind) -> Self {
        LengthDelimitedRecordsReaderBuilder {
            length_kind,
            inner,
            reader_builder: |it| CodedInputStream::new(it),
        }
        .build()
    }

    pub fn get_next(&mut self) -> Result<Vec<u8>, io::Error> {
        let length_kind = *self.borrow_length_kind();
        Ok(self.with_reader_mut(move |reader| {
            let len = match length_kind {
                LengthKind::BigEndianFixed => reader.read_u32::<BigEndian>()?,
                LengthKind::Varint => reader.read_raw_varint32()?,
            };

            let mut buf = vec![0; len as usize];
            <CodedInputStream as io::Read>::read_exact(reader, &mut buf)?;

            Ok::<_, io::Error>(buf)
        })?)
    }

    pub fn try_get_next(&mut self) -> Result<Option<Vec<u8>>, io::Error> {
        match self.get_next() {
            Ok(it) => Ok(Some(it)),
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
