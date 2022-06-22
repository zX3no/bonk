use blake3::*;
use jwalk::WalkDir;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    fs::{self, File},
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    //Collect all the files
    let mut files = collect_files();
    files.sort();

    //Hash each of them and save them
    let new_hash: Vec<String> = files.par_iter().map(|file| hash(file)).collect();

    //Compare the new hash with old hash
    let old_hash = fs::read_to_string("build/hash").unwrap_or(String::new());
    let new_hash = new_hash.join("\n");

    if new_hash != old_hash {
        if build().is_err() {
            return;
        };
        println!("Building...");
    }

    //Run the program
    let output = Command::new("build/main.exe").output().unwrap();

    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();

    //Save the new hash to the old hash
    fs::write("build/hash", new_hash).unwrap();
}

fn build() -> Result<(), ()> {
    //Read the build command from a file
    let crun = fs::read_to_string("crun").unwrap();
    let command: Vec<&str> = crun.split(' ').clone().collect();
    if command.len() < 1 {
        return Err(());
    }

    let child = Command::new(command[0])
        .args(command.get(1..).unwrap_or_default())
        .spawn()
        .unwrap();

    let output = child.wait_with_output().unwrap();

    if output.status.success() {
        Ok(())
    } else {
        Err(())
    }
}

fn hash(path: impl AsRef<Path>) -> String {
    let file = File::open(path).unwrap();
    let metadata = file.metadata().unwrap();
    let file_size = metadata.len();
    let map = unsafe {
        memmap2::MmapOptions::new()
            .len(file_size as usize)
            .map(&file)
            .unwrap()
    };

    let cursor = Cursor::new(map);
    let mut hasher = Hasher::new();
    hasher.update(cursor.get_ref());

    let mut output = hasher.finalize_xof();

    let mut block = [0; blake3::guts::BLOCK_LEN];
    let mut len = 32;
    let mut hex = String::new();
    while len > 0 {
        output.fill(&mut block);
        let hex_str = hex::encode(&block[..]);
        let take_bytes = std::cmp::min(len, block.len() as u64);
        hex.push_str(&hex_str[..2 * take_bytes as usize]);
        len -= take_bytes;
    }
    hex
}

fn collect_files() -> Vec<PathBuf> {
    WalkDir::new("src/")
        .into_iter()
        .flat_map(|dir| dir)
        .map(|dir| dir.path())
        .filter(|path| path.is_file())
        .collect()
}
