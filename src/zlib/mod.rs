use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Read, Write};

#[derive(Debug)]
pub enum ZlibCompressionLevel {
  None,
  Fast,
  Best,
  Default,
}

impl ZlibCompressionLevel {
  pub fn to_flate2_compression(&self) -> Compression {
      match self {
          ZlibCompressionLevel::None => Compression::none(),
          ZlibCompressionLevel::Fast => Compression::fast(),
          ZlibCompressionLevel::Best => Compression::best(),
          ZlibCompressionLevel::Default => Compression::default(),
      }
  }
}

pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}

pub fn compress_zlib(data: &[u8], level: ZlibCompressionLevel) -> Result<Vec<u8>, std::io::Error> {
    let level_flate = level.to_flate2_compression();
    let mut encoder = ZlibEncoder::new(Vec::new(), level_flate);
    encoder.write_all(data)?;
    encoder.finish()
}
