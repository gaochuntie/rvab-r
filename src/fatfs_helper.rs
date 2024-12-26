// src/fatfs_helper.rs
use crate::gpt_helper::bytes2ieee;
use fscommon::StreamSlice;
use std::io::prelude::*;

pub fn t_fatfs_format() {
    let fs_start: u64 = 2048; // example start sector
    let fs_end: u64 = 522239; // example end sector
    let sector_size: u64 = 512; // example sector size

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("test/disk.img")
        .expect("Error: open disk image failed");

    let offset = fs_start * sector_size;
    let length = (fs_end - fs_start + 1) * sector_size;

    let slice = StreamSlice::new(file, offset, length).expect("Error: create StreamSlice failed");
    let format_opt = fatfs::FormatVolumeOptions::new().volume_label(*b"RVAB       ");
    fatfs::format_volume(slice, format_opt).expect("Error: format volume failed");
}

pub fn t_fatfs_write() {
    let fs_start: u64 = 2048; // example start sector
    let fs_end: u64 = 522239; // example end sector
    let sector_size: u64 = 512; // example sector size
    let offset = fs_start * sector_size;
    let length = (fs_end - fs_start + 1) * sector_size;

    // Open the formatted volume
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("test/disk.img")
        .expect("Error: open disk image failed");
    let slice = StreamSlice::new(file, offset, length).expect("Error: create StreamSlice failed");
    let fs = fatfs::FileSystem::new(slice, fatfs::FsOptions::new().update_accessed_date(true))
        .expect("Error: open filesystem failed");

    // Write to a file
    let root_dir = fs.root_dir();
    let mut file = root_dir
        .create_file("test.txt")
        .expect("Error: create file failed");
    file.write_all(b"Hello, world!")
        .expect("Error: write to file failed");
}

pub fn t_fatfs_ls() {
    let fs_start: u64 = 2048; // example start sector
    let fs_end: u64 = 522239; // example end sector
    let sector_size: u64 = 512; // example sector size
    let offset = fs_start * sector_size;
    let length = (fs_end - fs_start + 1) * sector_size;

    // Open the formatted volume
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("test/disk.img")
        .expect("Error: open disk image failed");
    let slice = StreamSlice::new(file, offset, length).expect("Error: create StreamSlice failed");
    let fs = fatfs::FileSystem::new(slice, fatfs::FsOptions::new().update_accessed_date(true))
        .expect("Error: open filesystem failed");

    let root_dir = fs.root_dir();
    for r in root_dir.iter() {
        let e = r.expect("Error: read directory entry failed");
        let modified = e.modified();
        println!(
            "{:4}  {:?}  {}",
            bytes2ieee(e.len()),
            modified,
            e.file_name()
        );
    }
}

pub fn t_fatfs_cat() {
    let fs_start: u64 = 2048; // example start sector
    let fs_end: u64 = 522239; // example end sector
    let sector_size: u64 = 512; // example sector size
    let offset = fs_start * sector_size;
    let length = (fs_end - fs_start + 1) * sector_size;

    let filename = "test.txt";

    // Open the formatted volume
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("test/disk.img")
        .expect("Error: open disk image failed");
    let slice = StreamSlice::new(file, offset, length).expect("Error: create StreamSlice failed");
    let fs = fatfs::FileSystem::new(slice, fatfs::FsOptions::new().update_accessed_date(true))
        .expect("Error: open filesystem failed");

    let root_dir = fs.root_dir();
    let mut file = root_dir
        .open_file(&filename)
        .expect("Error: open file failed");
    let mut buf = vec![];
    file.read_to_end(&mut buf).expect("Error: read file failed");
    print!("{}", String::from_utf8_lossy(&buf));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore]
    fn test_fatfs_format() {
        t_fatfs_format();
    }
    #[test]
    #[ignore]
    fn test_fatfs_write() {
        t_fatfs_write();
    }
    #[test]
    fn test_fatfs_ls() {
        t_fatfs_ls();
    }
    #[test]
    fn test_fatfs_cat() {
        t_fatfs_cat();
    }
}
