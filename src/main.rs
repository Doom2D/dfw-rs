mod wad;
mod zlib;

use clap::{arg, command, Parser, Subcommand};
use std::env::current_dir;
use std::fs;
use std::fs::OpenOptions;
use std::fs::*;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
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
}

#[derive(Subcommand)]
enum Commands {
    /// extract dfwad contents to a folder
    Extract {},
    /// packs folder into a dfwad
    Pack {},
}

fn extract(source: &std::path::PathBuf, target: &std::path::PathBuf) {
    let file_path = source;
    // let file_name = file_path.file_name().unwrap().to_str().unwrap();
    let file_stem = file_path.file_stem().unwrap();
    let target_path = target.join(Path::new(&file_stem));
    println!("Path {}", target_path.display());
    fs::create_dir_all(target_path.clone()).unwrap();
    let mut data: Vec<u8> = Vec::new();
    let mut file = File::open(file_path).expect("Unable to open file");
    file.read_to_end(&mut data).expect("Unable to read data");
    let vec = parse_wad(&data).unwrap();
    println!("{}", target_path.display());
    for dir in vec.iter() {
        let dir_path = target_path
            .clone()
            .join(Path::new(&dir.dir.clone().replace('\0', "")));
        fs::create_dir_all(dir_path.clone()).unwrap();
        for elem in &dir.entries {
            let entry_path = dir_path
                .clone()
                .join(Path::new(&elem.name.clone().replace('\0', "")));
            println!("{}", entry_path.display());
            let bytes = read_entry(&data, &elem).unwrap();
            fs::write(entry_path, bytes).expect("Unable to write file");
        }
    }
}

fn parent_from_path(src: &std::path::Path, path: &std::path::Path) -> Result<String, ()> {
    let parent = path.parent().unwrap().strip_prefix(src).unwrap().to_str().unwrap().to_string();
    Ok(parent)
}

fn file_name_from_path(path: &std::path::Path) -> Result<String, ()> {
    let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
    Ok(file_name)
}

fn create_entry(src: &std::path::Path, path: &std::path::Path) -> Result<Entry, ()> {
    //println!("PatH: {}", src.display());
    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let dir = parent_from_path(src, path).unwrap();
    let name = file_name_from_path(path).unwrap();

    let entry = Entry { buffer, dir, name };
    Ok(entry)
}
/// one-below
fn create_entries(source: &std::path::Path, target: &std::path::Path) -> Result<Vec<EntryType>, ()> {
    let mut vec: Vec<EntryType> = Vec::new();
    for elem in WalkDir::new(source).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        // println!("Checking {}", elem.path().display());
        if elem.file_type().is_file() {
            let entry = create_entry(source, elem.path()).unwrap();
            vec.push(EntryType::Entry(entry));
        } else if elem.file_type().is_dir() {
            let elem_path = elem.path();
            for sub_elem in WalkDir::new(elem_path).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                //println!("Checking {}", sub_elem.path().display());
                if sub_elem.file_type().is_file() {
                    // let dir = sub_elem.path().strip_prefix(elem_path).unwrap();
                    // let name = sub_elem.path().file_name().unwrap();
                    let entry = create_entry(source, sub_elem.path()).unwrap();
                    vec.push(EntryType::Entry(entry));
                } else if sub_elem.file_type().is_dir() {
                    let entries = create_entries(sub_elem.path(), elem.path()).unwrap();
                    let nested = NestedEntry {
                        entries,
                        dir: elem.path().strip_prefix(source).unwrap().to_str().unwrap().to_string(),
                        name: sub_elem.path().file_name().unwrap().to_str().unwrap().to_string()
                    };
                    vec.push(EntryType::NestedEntry(nested))
                }
            }
        }
    }
    Ok(vec)
}

fn pack(source: &std::path::PathBuf, target: &std::path::PathBuf) -> Result<(), ()> {
    let source_dir = source;
    let source_as_path = source.as_path();
    let mut entries: Vec<EntryType> = Vec::new();
    for entry in WalkDir::new(source_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            // println!("ADding3: {}", entry.path().display());
            let entry = create_entry(source_as_path, entry.path()).unwrap();
            // println!("b: {} d: {} n:", entry.dir, entry.name);
            entries.push(EntryType::Entry(entry));
        } else if entry.file_type().is_dir() {
            let path = entry.path();
            for subentry in WalkDir::new(path).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                let subpath = subentry.path();
                // let mut subentries = Vec::new();
                if subentry.file_type().is_dir() {
                    for x in WalkDir::new(subpath).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                        let sub_wad_dir = subentry.path();
                        let mut sub_wad_entries = Vec::new();
                        if x.file_type().is_file() {
                            let subentry = create_entry(sub_wad_dir, x.path()).unwrap();
                            println!("ADding2: name: {} dir: {}", subentry.name, subentry.dir);
                            sub_wad_entries.push(EntryType::Entry(subentry));
                        } else {
                            for y in WalkDir::new(sub_wad_dir).min_depth(1).max_depth(2).into_iter().filter_map(|e| e.ok()) {
                                if y.file_type().is_file() {
                                    println!("Subentry path: {}", y.path().display());
                                }
                                else if y.file_type().is_dir() {
                                    // return Err(());
                                    println!("Is dir {}", y.path().display());
                                }
                            }
                            /* 
                            println!(
                                "{} {} {}",
                                entry.path().display(),
                                subentry.path().display(),
                                x.path().strip_prefix(sub_wad_dir).unwrap().display(),
                            );
                            */
                            // let subentry = create_entry(sub_wad_dir, x.path()).unwrap();
                            // println!("subentry: {}", subentry.dir);
                            /*
                            let nested = NestedEntry {
                                dir: parent_from_path(source_as_path, entry.path()).unwrap(),
                                name: file_name_from_path(entry.path()).unwrap(),
                                entries: subentries,
                            };
                            entries.push(EntryType::NestedEntry(nested));
                            */
                            // return Err(());
                            continue;
                        }
                        let sub_wad_entry = NestedEntry {
                            dir: subpath.strip_prefix(source_as_path).unwrap().parent().unwrap().to_str().unwrap().to_string(),
                            name: subpath.file_name().unwrap().to_str().unwrap().to_string(),
                            entries: sub_wad_entries,
                        };
                        entries.push(EntryType::NestedEntry(sub_wad_entry));
                    }
                } else if subentry.file_type().is_file() {
                    // println!("ADding1: {}", subentry.path().display());
                    let entry = create_entry(source_as_path, subentry.path()).unwrap();
                    // println!("b: {} d: {} n:", entry.dir, entry.name);
                    entries.push(EntryType::Entry(entry));
                }
            }
        }
    }
    //let entries = create_wad(&entries).unwrap();
    /*
    let bytes = create_wad2(&entries);
    println!("{}", target.display());
    let mut file = File::create(target.clone()).unwrap();
    file.write_all(&bytes.unwrap()).unwrap();
    */
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    // extract(&cli.source, &cli.target);
    // pack(&cli.source, &cli.target).unwrap();
    let res = create_entries(&cli.source, &cli.target).unwrap();
    for g in &res {
        match g {
            EntryType::Entry(entry) => {
                println!("name1: {} dir: {}", entry.name, entry.dir);
            }
            EntryType::NestedEntry(nested_entry) => {
                println!("name2: {} dir: {}", nested_entry.name, nested_entry.dir);
            }
        }
    }
    let bytes = create_wad2(&res).unwrap();
    let mut file = File::create(cli.target.clone()).unwrap();
    file.write_all(&bytes).unwrap()

    // println!("{:?}", entries);
}
