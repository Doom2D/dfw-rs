use crate::{zlib::*, Entry};

use std::io::{Read};
use std::str;
use encoding_rs::{WINDOWS_1251};
use std::fs::*;
use std::path::*;
use std::io::Seek;
use std::io::Write;
use std::io::SeekFrom;
use std::collections::HashSet;

const DFWAD_SIGNATURE: &str = "DFWAD";
const DFWAD_SUPPORTED_VERSION: u8 = 1;
const DFWAD_STRUCT_NAME_BYTES: usize = 16;
const DFWAD_STRUCT_OFFSET_BYTES: usize = 4;
const DFWAD_STRUCT_SIZE_BYTES: usize = 4;


#[derive(Debug)]
pub struct WadEntry {
  pub name: String,
  pub offset: u32,
  pub size: u32,
}

#[derive(Debug)]
pub struct WadDirectory {
  pub dir: String,
  pub entries: Vec<WadEntry>
}

impl WadEntry {
  fn new(name: &str, size: u32, offset: u32) -> WadEntry {
      WadEntry {
          name: name.to_string(),
          size: size,
          offset: offset,
      }
  }
}

impl WadDirectory {
  fn new(dir: &str, entries: Vec<WadEntry>) -> WadDirectory {
    WadDirectory { dir: dir.to_string(), entries }
  }
}

#[derive(Debug)]
pub enum WadError {
  IncorrectSignature,
  UnsupportedVersion,
  InvalidEntry,
  StructNameTooLong,
  EmptyDirectory
}

pub fn parse_wad(data: &Vec<u8>) -> Result<Vec<WadDirectory>, WadError> {
  let mut cursor = std::io::Cursor::new(data);
  let mut signature_buffer = [0; 5];
  let mut version_buffer = [0; 1];
  let mut number_of_lumps_buffer = [0; 2];
  cursor.read_exact(&mut signature_buffer).unwrap();
  cursor.read_exact(&mut version_buffer).unwrap();
  cursor.read_exact(&mut number_of_lumps_buffer).unwrap();
  let signature = str::from_utf8(&signature_buffer).unwrap();
  let version: u8 = version_buffer[0];
  let number_of_lumps = u16::from_le_bytes(number_of_lumps_buffer);
  if signature != DFWAD_SIGNATURE {
      return Err(WadError::IncorrectSignature);
  } else if version != DFWAD_SUPPORTED_VERSION {
      return Err(WadError::UnsupportedVersion);
  }
  let mut current_directory: String = "".to_string();
  let mut entries: Vec<WadEntry> = Vec::new();
  let mut directories: Vec<WadDirectory> = Vec::new();
  for _ in 0..number_of_lumps {
      let mut struct_name_buffer = [0u8; 16];
      let mut offset_buffer = [0u8; 4];
      let mut length_buffer = [0u8; 4];
      cursor.read_exact(&mut struct_name_buffer).unwrap();
      cursor.read_exact(&mut offset_buffer).unwrap();
      cursor.read_exact(&mut length_buffer).unwrap();
      let (struct_name, _, _) = WINDOWS_1251.decode(&struct_name_buffer);
      let offset = u32::from_le_bytes(offset_buffer);
      let length = u32::from_le_bytes(length_buffer);
      if offset == 0 && length == 0 {
          if entries.len() != 0 {
            directories.push(WadDirectory::new(&current_directory, entries));
          }
          current_directory = struct_name.to_string();
          entries = Vec::new();
          continue;
      }
      if (offset == 0 && length != 0) || (offset != 0 && length == 0) {
          return Err(WadError::InvalidEntry);
      }
      let entry = WadEntry::new(&struct_name.to_string().replace('\0', ""), length, offset);
      println!("Entry {}, offset: {}, length: {}", entry.name, entry.offset, entry.size);
      entries.push(entry);
  }
  println!("DFWAD {} with signature {} has {} entries", signature, version, number_of_lumps);
  Ok(directories)
}

pub fn create_wad2(data: &Vec<Entry>) -> Result<Vec<u8>, WadError> {
  let mut sum: usize = 0;
  let dirs: HashSet<String> = data.iter()
    .map(|d| d.dir.clone())
    .collect();
  let mut entry_vectors: Vec<Vec<Entry>> = Vec::new();
  // let dirs: Vec<WadDirectory> = Vec::new();
  for dir in dirs {
    println!("dir: {}", dir);
    sum = sum + 1;
    // let entries = data.clone().iter().filter(|d| d.dir == dir).collect;
    let entries: Vec<Entry> = data.clone().iter().cloned().filter(|d| d.dir == dir).collect();
    sum = sum + entries.len();
    entry_vectors.push(entries);
  }

  println!("sum: {} len: {}", sum, data.len());

  let bytes: Vec<u8> = Vec::new();
  let mut cursor = std::io::Cursor::new(bytes);

  let (magic_bytes, _, _) = WINDOWS_1251.encode(DFWAD_SIGNATURE);
  let version_bytes = [DFWAD_SUPPORTED_VERSION];
  let number_of_entries_bytes = sum as u16;
  cursor.write_all(&magic_bytes).unwrap();
  cursor.write_all(&version_bytes).unwrap();
  cursor.write_all(&number_of_entries_bytes.to_le_bytes()).unwrap();

  let wad_header_offset: usize = 5 + 1 + 2;
  let entry_start = wad_header_offset;
  let entries_final_offset = entry_start + (sum * (DFWAD_STRUCT_NAME_BYTES + DFWAD_STRUCT_OFFSET_BYTES + DFWAD_STRUCT_SIZE_BYTES));
  let data_start = entries_final_offset;

  let mut entry_offset = entry_start;
  let mut data_offset = data_start;   

  for v in entry_vectors {
    if v.len() == 0 {
      return Err(WadError::EmptyDirectory);
    }
    let dir_offset_buffer = [0u8; DFWAD_STRUCT_OFFSET_BYTES];
    let dir_length_buffer = [0u8; DFWAD_STRUCT_SIZE_BYTES];
    let dir_name_unencoded = v[0].dir.clone();
    let (dir_name_bytes, _, _) = WINDOWS_1251.encode(&dir_name_unencoded);
    let mut padded_dir_name_bytes = Vec::from(dir_name_bytes);
    padded_dir_name_bytes.resize(DFWAD_STRUCT_NAME_BYTES, 0);
    // cursor.seek(SeekFrom::Start(entry_offset as u64)).unwrap();
    cursor.write_all(&padded_dir_name_bytes).unwrap();
    cursor.write_all(&dir_offset_buffer).unwrap();
    cursor.write_all(&dir_length_buffer).unwrap();
    for entry in v {
      let compressed = compress_zlib(&entry.buffer, ZlibCompressionLevel::Default).unwrap();
      let size = compressed.len() as usize;
      // let path = Path::new(&entry.dir);
      // let name = path.file_name().unwrap().to_str().unwrap();
      println!("{} {}", &entry.name, &entry.dir);
      let (struct_name_bytes, _, _) = WINDOWS_1251.encode(&entry.name);
      let mut padded_struct_name_bytes = Vec::from(struct_name_bytes);
      padded_struct_name_bytes.resize(DFWAD_STRUCT_NAME_BYTES, 0);

      // cursor.seek(SeekFrom::Start(entry_offset as u64)).unwrap();
      // println!("{}", padded_struct_name_bytes.len());
      cursor.write_all(&padded_struct_name_bytes).unwrap();
      cursor.write_all(&(data_offset as u32).to_le_bytes()).unwrap();
      cursor.write_all(&(size as u32).to_le_bytes()).unwrap();
      cursor.seek(SeekFrom::Start(data_offset as u64)).unwrap();
      cursor.write_all(&compressed).unwrap();
      data_offset = data_offset + size;
      entry_offset = entry_offset + DFWAD_STRUCT_NAME_BYTES + DFWAD_STRUCT_OFFSET_BYTES + DFWAD_STRUCT_SIZE_BYTES;
      cursor.seek(SeekFrom::Start(entry_offset as u64)).unwrap();
    }
  }

  Ok(cursor.into_inner())
}

pub fn create_wad(data: &Vec<Entry>) -> Result<Vec<u8>, WadError> {
  let mut sum: usize = 0;
  let dirs: HashSet<String> = data.iter()
    .map(|d| d.dir.clone())
    .collect();
  let mut entry_vectors: Vec<Vec<Entry>> = Vec::new();
  // let dirs: Vec<WadDirectory> = Vec::new();
  for dir in dirs {
    println!("dir: {}", dir);
    sum = sum + 1;
    // let entries = data.clone().iter().filter(|d| d.dir == dir).collect;
    let entries: Vec<Entry> = data.clone().iter().cloned().filter(|d| d.dir == dir).collect();
    sum = sum + entries.len();
    entry_vectors.push(entries);
  }

  println!("sum: {} len: {}", sum, data.len());
  
  let bytes: Vec<u8> = Vec::new();
  let mut cursor = std::io::Cursor::new(bytes);

  let (magic_bytes, _, _) = WINDOWS_1251.encode(DFWAD_SIGNATURE);
  let version_bytes = [DFWAD_SUPPORTED_VERSION];
  let number_of_entries_bytes = sum as u16;
  cursor.write_all(&magic_bytes).unwrap();
  cursor.write_all(&version_bytes).unwrap();
  cursor.write_all(&number_of_entries_bytes.to_le_bytes()).unwrap();

  let wad_header_offset: usize = 5 + 1 + 2;

  let entry_start = wad_header_offset;
  let mut entry_offset = entry_start;
  let entries_final_offset = entry_start + (sum * (DFWAD_STRUCT_NAME_BYTES + DFWAD_STRUCT_OFFSET_BYTES + DFWAD_STRUCT_SIZE_BYTES));
  let data_start = entries_final_offset;
  let mut data_offset = data_start;

  for v in entry_vectors {
    if v.len() == 0 {
      return Err(WadError::EmptyDirectory);
    }
    let dir_offset_buffer = [0u8; DFWAD_STRUCT_OFFSET_BYTES];
    let dir_length_buffer = [0u8; DFWAD_STRUCT_SIZE_BYTES];
    let dir_name_unencoded = v[0].name.clone();
    let (dir_name_bytes, _, _) = WINDOWS_1251.encode(&dir_name_unencoded);
    let mut padded_dir_name_bytes = Vec::from(dir_name_bytes);
    padded_dir_name_bytes.resize(DFWAD_STRUCT_NAME_BYTES, 0);
    cursor.seek(SeekFrom::Start(entry_offset as u64)).unwrap();
    cursor.write_all(&padded_dir_name_bytes).unwrap();
    cursor.write_all(&dir_offset_buffer).unwrap();
    cursor.write_all(&dir_length_buffer).unwrap();
    entry_offset = entry_offset + DFWAD_STRUCT_NAME_BYTES + DFWAD_STRUCT_OFFSET_BYTES + DFWAD_STRUCT_SIZE_BYTES;
    for entry in v {
      let compressed = compress_zlib(&entry.buffer, ZlibCompressionLevel::Default).unwrap();
      let size = compressed.len() as usize;
      let (struct_name_bytes, _, _) = WINDOWS_1251.encode(&entry.dir);
      let mut padded_struct_name_bytes = Vec::from(struct_name_bytes);
      padded_struct_name_bytes.resize(DFWAD_STRUCT_NAME_BYTES, 0);

      cursor.seek(SeekFrom::Start(entry_offset as u64)).unwrap();
      cursor.write_all(&padded_struct_name_bytes).unwrap();
      cursor.write_all(&(data_offset as u32).to_le_bytes()).unwrap();
      cursor.write_all(&(size as u32).to_le_bytes()).unwrap();
      cursor.seek(SeekFrom::Start(data_offset as u64)).unwrap();
      cursor.write_all(&compressed).unwrap();

      data_offset = data_offset + size;
      entry_offset = entry_offset + DFWAD_STRUCT_NAME_BYTES + DFWAD_STRUCT_OFFSET_BYTES + DFWAD_STRUCT_SIZE_BYTES;
    }
  }

  Ok(cursor.into_inner())
}

pub fn pack_wad1(data: &Vec<WadDirectory>) -> Result<Vec<u8>, WadError> {
  let mut unique_dirs = HashSet::<String>::new();

  for entry in data {
    unique_dirs.insert(entry.dir.clone());
  }
  let number_of_entries = unique_dirs.len() + data.len();

  let (magic_bytes, _, _) = WINDOWS_1251.encode(DFWAD_SIGNATURE);
  let version_bytes = [DFWAD_SUPPORTED_VERSION];
  let number_of_entries_bytes = number_of_entries as u16;

  let bytes: Vec<u8> = Vec::new();
  let mut cursor = std::io::Cursor::new(bytes);
  cursor.write_all(&magic_bytes).unwrap();
  cursor.write_all(&version_bytes).unwrap();
  cursor.write_all(&number_of_entries_bytes.to_le_bytes()).unwrap();
  println!("{:?}", unique_dirs);
/* 
  let mut entry_offset = cursor.position() as u32;
  let data_start = cursor.position() as u32 + (number_of_entries as u32 * (DFWAD_STRUCT_NAME_BYTES as u32 + DFWAD_STRUCT_OFFSET_BYTES as u32 + DFWAD_STRUCT_SIZE_BYTES as u32));
  let mut data_offset: u32 = data_start;

  data.sort_by_key(|entry| entry.dir.clone());
  for entry in data {
    let compressed = compress_zlib(&entry.buffer, ZlibCompressionLevel::Default).unwrap();
    let size = compressed.len() as u32;
    let (struct_name_bytes, _, _) = WINDOWS_1251.encode(&entry.dir);
    entry_offset = entry_offset + DFWAD_STRUCT_NAME_BYTES as u32 + DFWAD_STRUCT_OFFSET_BYTES as u32 + DFWAD_STRUCT_SIZE_BYTES as u32;
    data_offset = size + data_offset;
    cursor.write_all(&struct_name_bytes).unwrap();
    cursor.write_all(&data_offset.to_le_bytes()).unwrap();
    cursor.write_all(&size.to_le_bytes()).unwrap();
    cursor.seek(SeekFrom::Start(data_offset as u64)).unwrap();
    cursor.write_all(&compressed).unwrap();
    cursor.seek(SeekFrom::Start(entry_offset as u64)).unwrap();
  }
*/
  for dir in unique_dirs {
    let offset_buffer = [0u8; DFWAD_STRUCT_OFFSET_BYTES];
    let length_buffer = [0u8; DFWAD_STRUCT_SIZE_BYTES];
    let (struct_name_bytes, _, _) = WINDOWS_1251.encode(&dir);
    if struct_name_bytes.len() > DFWAD_STRUCT_NAME_BYTES {
      return Err(WadError::StructNameTooLong);
    }
    let mut padded_struct_name_bytes = Vec::from(struct_name_bytes);
    padded_struct_name_bytes.resize(DFWAD_STRUCT_NAME_BYTES, 0);
    println!("{}", padded_struct_name_bytes.len());
    cursor.write_all(&padded_struct_name_bytes).unwrap();
    cursor.write_all(&offset_buffer).unwrap();
    cursor.write_all(&length_buffer).unwrap();
  }

  Ok(cursor.into_inner())
}

pub fn read_entry(data: &Vec<u8>, entry: &WadEntry) -> Result<Vec<u8>, WadError> {
  let mut cursor = std::io::Cursor::new(data);
  cursor.seek(SeekFrom::Start(entry.offset as u64)).ok();
  let mut entry_data_compressed = vec![0u8; entry.size as usize];
  cursor.read_exact(&mut entry_data_compressed).ok();
  let entry_data_uncompressed = decompress_zlib(&entry_data_compressed).unwrap();
  Ok(Vec::from(entry_data_uncompressed))
}