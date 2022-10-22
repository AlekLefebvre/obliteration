use self::entry::Entry;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::os::raw::c_char;
use std::path::Path;
use std::ptr::null_mut;
use util::mem::{read_array, read_u16_le};

pub mod compressed;
pub mod entry;
pub mod uncompressed;

#[no_mangle]
pub extern "C" fn pup_open(file: *const c_char, err: *mut *mut error::Error) -> *mut Pup {
    let file = util::str::from_c_unchecked(file);
    let pup = match Pup::open(file) {
        Ok(v) => Box::new(v),
        Err(e) => {
            unsafe { *err = error::Error::new(&e) };
            return null_mut();
        }
    };

    Box::into_raw(pup)
}

#[no_mangle]
pub extern "C" fn pup_free(pup: *mut Pup) {
    unsafe { Box::from_raw(pup) };
}

pub struct Pup {
    file: memmap2::Mmap,
    entries: Vec<Entry>,
    table_entries: Vec<Option<usize>>,
}

impl Pup {
    pub fn open<F: AsRef<Path>>(file: F) -> Result<Self, OpenError> {
        // Open file and map it to memory.
        let file = match File::open(file) {
            Ok(v) => v,
            Err(e) => return Err(OpenError::OpenFailed(e)),
        };

        let file = match unsafe { memmap2::Mmap::map(&file) } {
            Ok(v) => v,
            Err(e) => return Err(OpenError::MapFailed(e)),
        };

        if file.len() < 32 {
            return Err(OpenError::TooSmall);
        }

        // Check magic.
        let hdr = file.as_ptr();
        let magic: [u8; 4] = read_array(hdr, 0);

        if magic != [0x4f, 0x15, 0x3d, 0x1d] {
            return Err(OpenError::InvalidMagic);
        }

        // Read entry headers.
        let entry_count = read_u16_le(hdr, 24) as usize;
        let mut entries: Vec<Entry> = Vec::with_capacity(entry_count);

        for i in 0..entry_count {
            let offset = 32 + i * Entry::RAW_SIZE;
            let entry = match file.get(offset..(offset + Entry::RAW_SIZE)) {
                Some(v) => Entry::read(v.as_ptr()),
                None => return Err(OpenError::TooSmall),
            };

            entries.push(entry);
        }

        // TODO: What is table?
        let mut table_entries: Vec<Option<usize>> = vec![None; entries.len()];

        for i in 0..entries.len() {
            let entry = &entries[i];

            if entry.is_blocked() {
                if ((entry.id() | 0x100) & 0xf00) == 0xf00 {
                    // What is this?
                    todo!();
                }

                let table = entries
                    .iter()
                    .position(|e| (e.flags() & 1) != 0 && (e.id() as usize) == i)
                    .unwrap();

                if table_entries[table].is_some() {
                    // What is this?
                    todo!();
                }

                table_entries[table] = Some(i);
            } else {
                table_entries[i] = None;
            }
        }

        Ok(Self {
            file,
            entries,
            table_entries,
        })
    }

    pub fn get_system_image(&self) -> Option<Box<dyn Read + '_>> {
        for i in 0..self.entries.len() {
            let entry = &self.entries[i];
            let special = entry.flags() & 0xf0000000;

            if special == 0xe0000000 || special == 0xf0000000 || self.table_entries[i].is_some() {
                continue;
            }

            if entry.id() == 6 {
                return Some(self.create_reader(entry));
            }
        }

        None
    }

    fn create_reader<'a>(&'a self, entry: &'a Entry) -> Box<dyn Read + 'a> {
        if entry.is_compressed() {
            Box::new(compressed::Reader::new(entry, &self.file))
        } else {
            Box::new(uncompressed::Reader::new(entry, &self.file))
        }
    }
}

#[derive(Debug)]
pub enum OpenError {
    OpenFailed(std::io::Error),
    MapFailed(std::io::Error),
    TooSmall,
    InvalidMagic,
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OpenFailed(e) | Self::MapFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::OpenFailed(_) => f.write_str("cannot open file"),
            Self::MapFailed(_) => f.write_str("cannot map file"),
            Self::TooSmall => f.write_str("file too small"),
            Self::InvalidMagic => f.write_str("invalid magic"),
        }
    }
}
