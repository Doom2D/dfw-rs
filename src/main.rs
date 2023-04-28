mod wad;
mod zlib;
use clap::{arg, command, Parser, Subcommand};
use zlib::ZlibCompressionLevel;
use std::fs;
use std::fs::*;
use std::io::Read;
use std::io::Write;
use std::path::*;
use wad::*;
use walkdir::WalkDir;
#[derive(Debug, Clone)]
pub struct Entry {
    buffer: Vec<u8>,
    dir: String,
    name: String,
}

#[derive(Debug, Clone)]
pub struct NestedEntry {
    dir: String,
    name: String,
    entries: Vec<EntryType>,
}

#[derive(Debug, Clone)]
pub enum EntryType {
    Entry(Entry),
    NestedEntry(NestedEntry),
}

#[derive(Debug)]
pub enum Action {
    CheckIfFile,
    TraverseDirectory,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    /// Which action dfwad should take
    command: Commands,

    /// File or directory to act upon
    source: std::path::PathBuf,

    /// Target file or directory
    target: std::path::PathBuf,

    /// Enable verbose mode
    #[arg(short, long)]
    verbose: bool,

    /// Zlib compression level to use for DFWADs
    #[arg(short, long)]
    #[clap(value_enum, default_value_t=ZlibCompressionLevel::Default)]
    zlib_level: ZlibCompressionLevel
}

#[derive(Subcommand)]
enum Commands {
    /// extract dfwad contents to a folder
    Extract {},
    /// packs folder into a dfwad
    Pack {},
}

fn extract_from_bytes(source: Vec<u8>, target: &std::path::Path, verbose: bool) {
    let vec = parse_wad(&source).unwrap();
    for dir in vec.iter() {
        let dir_path = target
            .clone()
            .join(Path::new(&dir.dir.clone().replace('\0', "")));
        if verbose {
            println!("{}:", dir_path.file_name().unwrap().to_str().unwrap());
        }
        fs::create_dir_all(dir_path.clone()).unwrap();
        for elem in &dir.entries {
            let entry_path = dir_path
                .clone()
                .join(Path::new(&elem.name.clone().replace('\0', "")));
            if verbose {
                println!("{}", entry_path.file_name().unwrap().to_str().unwrap());
            }
            let bytes = read_entry(&source, &elem).unwrap();
            if is_wad_signature(&bytes) {
                extract_from_bytes(bytes, &entry_path, verbose);
            } else {
                fs::write(entry_path, bytes).expect("Unable to write file");
            }
        }
    }
}

fn extract(source: &std::path::PathBuf, target: &std::path::PathBuf, verbose: bool) {
    let file_path = source;
    fs::create_dir_all(target.clone()).unwrap();
    let mut data: Vec<u8> = Vec::new();
    let mut file = File::open(file_path).expect("Unable to open file");
    file.read_to_end(&mut data).expect("Unable to read data");
    extract_from_bytes(data, target.as_path(), verbose);
}

fn parent_from_path(src: &std::path::Path, path: &std::path::Path) -> Result<String, ()> {
    let parent = path
        .parent()
        .unwrap()
        .strip_prefix(src)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    Ok(parent)
}

fn file_name_from_path(path: &std::path::Path) -> Result<String, ()> {
    let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
    Ok(file_name)
}

fn create_entry(src: &std::path::Path, path: &std::path::Path) -> Result<Entry, ()> {
    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let dir = parent_from_path(src, path).unwrap();
    let name = file_name_from_path(path).unwrap();

    let entry = Entry { buffer, dir, name };
    Ok(entry)
}
/// one-below
fn create_entries(
    source: &std::path::Path,
    _target: &std::path::Path,
) -> Result<Vec<EntryType>, ()> {
    let mut vec: Vec<EntryType> = Vec::new();
    for elem in WalkDir::new(source)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if elem.file_type().is_file() {
            let entry = create_entry(source, elem.path()).unwrap();
            vec.push(EntryType::Entry(entry));
        } else if elem.file_type().is_dir() {
            let elem_path = elem.path();
            for sub_elem in WalkDir::new(elem_path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if sub_elem.file_type().is_file() {
                    let entry = create_entry(source, sub_elem.path()).unwrap();
                    vec.push(EntryType::Entry(entry));
                } else if sub_elem.file_type().is_dir() {
                    let entries = create_entries(sub_elem.path(), elem.path()).unwrap();
                    let nested = NestedEntry {
                        entries,
                        dir: elem
                            .path()
                            .strip_prefix(source)
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                        name: sub_elem
                            .path()
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                    };
                    vec.push(EntryType::NestedEntry(nested))
                }
            }
        }
    }
    Ok(vec)
}

fn pack(source: &std::path::PathBuf, target: &std::path::PathBuf, verbose: bool, level: ZlibCompressionLevel) -> Result<(), ()> {
    let res = create_entries(&source, &target).unwrap();
    if verbose {
        for g in &res {
            match g {
                EntryType::Entry(entry) => {
                    println!("{}/{}", entry.dir, entry.name);
                }
                EntryType::NestedEntry(nested_entry) => {
                    println!("{}/{}", nested_entry.dir, nested_entry.name);
                }
            }
        }
    }
    let bytes = create_wad(&res, level).unwrap();
    let mut file = File::create(target.clone()).unwrap();
    file.write_all(&bytes).unwrap();
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Extract {} => {
            extract(&cli.source, &cli.target, cli.verbose);
        }
        Commands::Pack {} => {
            pack(&cli.source, &cli.target, cli.verbose, cli.zlib_level).unwrap();
        }
    };
}
