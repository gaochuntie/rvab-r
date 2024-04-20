use std::collections::{HashMap, HashSet};
use std::{fs, io};
use std::path::{Path, PathBuf};
use gpt::disk::LogicalBlockSize;
use gpt::GptConfig;
use crate::backup_factory::{BackupTrait, BackupType};
use crate::constants::*;
use crate::metadata::*;
use crate::math_support::*;

/// Get the userdata driver path
/// panic if not found
pub fn get_userdata_driver() -> String {
    let emmc_path = EMMC_TRAIT_FILE;
    if fs::metadata(emmc_path).is_ok() {
        //emmc
        return emmc_path.to_string();
    }
    let mut userdata_link_path = USERDATA_LINK_PATH;
    if fs::metadata(userdata_link_path).is_ok() == false {
        //
        userdata_link_path = USERDATA_LINK_PATH_BOOT;
    }
    if fs::metadata(userdata_link_path).is_ok() == false {
        //
        userdata_link_path = USERDATA_LINK_PATH_PLATFORM;
    }
    if fs::metadata(userdata_link_path).is_ok() == false {
        panic!("Error: userdata driver not found");
    }
    // Get the target of the symbolic link
    let target_path = match fs::read_link(userdata_link_path) {
        Ok(target) => target.to_str().unwrap().to_string(),
        Err(e) => {
            panic!("Error reading symbolic link: {:?}", e);
        }
    };
    // Iterate from sda to sdz
    for c in 'a'..='z' {
        let potential_path = format!("/dev/block/sd{}", c);

        if target_path.contains(&potential_path) {
            return potential_path.to_string();
        }
    }
    panic!("Error: userdata driver not found");
}

/// Find the largest free space tuple,args (start_lba,length_lba) return (start_lba, END_lba) in the disk
pub fn find_max_free_tuple(tuples: &Vec<(u64, u64)>) -> (u64, u64) {
    let mut max_size: u64 = 0;
    let mut max_start: u64 = 0;
    let mut max_end: u64 = 0;
    for (start, length) in tuples {
        if *length > max_size {
            max_size = *length;
            max_start = *start;
            max_end = *start + *length - 1;
        }
    }
    (max_start, max_end)
}

/// Auto layout for two userdata
/// Default space strategy: peace split
/// Default backuptype: test order: losetup,partition,binaryspace
/// Default backup_target: follow the userdata
/// panic if no backup type available or no enough space
pub fn auto_layout_freespace_example(start_lba: u64, end_lba: u64, sector: u64, ex_back_fpath: &Option<String>, dual_list: &Option<String>, _back_min_size_sector: &mut u64) -> (Slot, Slot) {
    //half split
    let p1_size = (end_lba - start_lba + 1) / 2;
    let p2_size = (end_lba - start_lba + 1) - p1_size;
    let p1_end = start_lba + p1_size - 1;
    let p2_start = p1_end + 1;
    // exclude file list
    let mut exclude_files = HashSet::new();
    let mut dual_files = HashSet::new();
    if let Some(path) = ex_back_fpath {
        //read exclude file and add into exclude list
        let file = fs::read_to_string(path).expect("Error reading exclude list file");
        let files: Vec<&str> = file.lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect();
        for item in files {
            exclude_files.insert(item.to_string());
        }
    };
    if let Some(path) = dual_list {
        //read exclude file and add into exclude list
        let file = fs::read_to_string(path).expect("Error reading dual list file");
        let files: Vec<&str> = file.lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect();
        for item in files {
            exclude_files.insert(item.to_string());
            dual_files.insert(item.to_string());
        }
    }
    let (_fw_num, fw_size) = calculate_firmware_size(&exclude_files);

    //test partition backup
    let mut backup_type = BackupType::Losetup;
    if let Err(_) = backup_type.test(None) {
        backup_type = BackupType::Partition;
    }
    if let Err(_) = backup_type.test(None) {
        backup_type = BackupType::BinarySpace;
    }
    if let Err(_) = backup_type.test(None) {
        panic!("Error: no backup type available");
    }
    let back_min_size_sector = backup_type.guess_backup_target_partition_size_sector(fw_size, sector);
    *_back_min_size_sector = back_min_size_sector;
    //construct slots
    let mut userdata1_start_lba = calculate_metadata_interval(start_lba, sector).1;
    //alignment 2 boundary
    if userdata1_start_lba % 2 == 1 {
        userdata1_start_lba += 1;
    };
    let mut userdata2_start_lba = calculate_metadata_interval(p2_start, sector).1;
    //alignment 2 boundary
    if userdata2_start_lba % 2 == 1 {
        userdata2_start_lba += 1;
    };
    let backup_target = "none".to_string();
    ///test space layout is correct for partition backup and binaryspace backup and losetup backup
    if back_min_size_sector > p2_size {
        panic!("Error: no enough space for backup target {}", bytes2ieee(back_min_size_sector * sector));
    }
    if (p1_end - back_min_size_sector + 1 - (USERDATA_MIN_SIZE / sector) - (METADATA_SIZE_BYTES / sector)) < start_lba {
        panic!("Error: no enough space for userdata1");
    };
    if (end_lba - back_min_size_sector + 1 - (USERDATA_MIN_SIZE / sector) - (METADATA_SIZE_BYTES / sector)) < p2_start {
        panic!("Error: no enough space for userdata2");
    };
    //userdata1
    let userdata1 = PartitionRawTarget {
        part_name: USERDATA_NAME.to_string(),
        driver: get_userdata_driver(),
        start_lba: userdata1_start_lba,
        end_lba: p1_end - back_min_size_sector,
    };
    let mut map1 = HashMap::new();
    map1.insert(USERDATA_NAME.to_string(), userdata1);
    let slot1: Slot = Slot {
        slot_name: "a".to_string(),
        backup_type_code: BackupType::type2code(backup_type.clone()),
        backup_target: backup_target.clone(),
        backup_exclude_list: exclude_files.clone(),
        backup_target_start: p1_end - back_min_size_sector + 1,
        backup_target_end: p1_end,
        backup_target_attr: "".to_string(),
        dyn_partition_set: map1,
    };
    //userdata2
    let userdata2 = PartitionRawTarget {
        part_name: USERDATA_NAME.to_string(),
        driver: get_userdata_driver(),
        start_lba: userdata2_start_lba,
        end_lba: end_lba - back_min_size_sector,
    };
    let mut map2 = HashMap::new();
    map2.insert(USERDATA_NAME.to_string(), userdata2);
    let slot2: Slot = Slot {
        slot_name: "b".to_string(),
        backup_type_code: BackupType::type2code(backup_type),
        backup_target,
        backup_exclude_list: exclude_files,
        backup_target_start: end_lba - back_min_size_sector + 1,
        backup_target_end: end_lba,
        backup_target_attr: "".to_string(),
        dyn_partition_set: map2,
    };
    (slot1, slot2)
}

/// Calculate the size of the firmwares,return in (total_num,total_bytes)
/// include all physical partitions under block/by-name/ except userdata
/// panic if block device dir access error occurs or total size=0
pub fn calculate_firmware_size(ex_back_list: &HashSet<String>) -> (u64, u64) {
    let mut firmware_size: u64 = 0;
    let dev_dir = get_block_dev_dir();
    let mut total_num = 0;
    let files = fs::read_dir(&dev_dir).unwrap();
    let mut exclude_files = get_block_dev_filenames();
    // merge ex_back_list into exclude_files

    for item in ex_back_list {
        exclude_files.insert(item.clone());
    };

    for file in files {
        let path = file.unwrap().path();
        let path_str = path.to_str().unwrap();
        let filename = &path.file_name().unwrap().to_str().unwrap().to_string();
        let global_exclude_files = &BACK_EXCLUDE_LIST;
        // Skip if the file is in the exclude list
        if exclude_files.contains(filename) || global_exclude_files.iter().any(|&x| filename.contains(x)) {
            println!("skip excluded file {}", &path_str);
            continue;
        };
        let metadata = fs::metadata(&path).unwrap();
        //TODO guess skip subdir, usually no subdir
        if metadata.is_dir() {
            //println!("skip subdir {}", &path_str);
            continue;
        }
        //TODO skip non-symbloic link , std not work
        let target_file = fs::read_link(&path).unwrap();

        firmware_size += read_block_dev_size(&target_file);
        total_num += 1;
        //println!("{} firmware_size:{}", target_file.to_str().unwrap(), bytes2ieee(&firmware_size));
    }
    if firmware_size == 0 {
        panic!("Error: firmware size=0");
    }
    println!("firmware_size:{}", bytes2ieee(firmware_size));
    (total_num, firmware_size)
}

/// read block device size via linux kernel, return bytes
/// by reading /sys/class/block/dev_node/size
pub fn read_block_dev_size(dev_path: &PathBuf) -> u64 {
    let dev_name = dev_path.file_name().unwrap().to_str().unwrap();
    //read size
    let size_path = format!("/sys/class/block/{}/size", dev_name);
    let size_str = fs::read_to_string(size_path).expect("Error reading size file");
    let size: u64 = size_str.trim().parse().expect("Error parsing size");
    size * 512
}

/// get partition main driver, path can be link or real device
pub fn get_partition_main_driver(spath: &str) -> Result<String, &'static str> {
    let emmc_path = EMMC_TRAIT_FILE;
    if fs::metadata(emmc_path).is_ok() {
        //emmc
        return Ok(emmc_path.to_string());
    }

    let path = Path::new(spath);
    if fs::metadata(path).is_ok() == false {
        return Err("Error: no such partition file");
    }
    let mut dev_node;
    match nix::fcntl::readlink(path) {
        Ok(node) => {
            dev_node = node.to_str().unwrap().to_string();
        }
        Err(_) => {
            dev_node = spath.to_string();
        }
    }
    // Iterate from sda to sdz
    for c in 'a'..='z' {
        let potential_path = format!("/dev/block/sd{}", c);

        if dev_node.contains(&potential_path) {
            return Ok(potential_path.to_string());
        }
    }
    return Err("Error: partition driver not found");
}

///Takes a size and converts this to a size in IEEE-1541-2002 units (KiB, MiB, GiB, TiB, PiB, or EiB),precision 1
pub fn bytes2ieee(size: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    let mut size = size.clone() as f64;
    let mut i = 0;
    while size >= 1024.0 && i < units.len() - 1 {
        size /= 1024.0;
        i += 1;
    }
    format!("{:.1}{}", size, units[i])
}

/// get  all block dev filenam list
pub fn get_block_dev_filenames() -> HashSet<String> {
    let path = BLOCK_DEV_DIR;
    let mut exclude_files = HashSet::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry {
                // If the entry is a file (not a directory), add its name to the list
                if let Ok(metadata) = entry.metadata() {
                    if !metadata.is_dir() {
                        if let Some(filename) = entry.file_name().to_str() {
                            exclude_files.insert(filename.to_string());
                        }
                    }
                }
            }
        }
    }

    exclude_files
}

/// get part accelerate location via gpt table,return (main_driver,id_num,first_lba,last_lba,sector_size)
pub fn get_part_accelerate_location(part_name: &str) -> Result<(String, u32, u64, u64, u64), &'static str> {
    let path = format!("{}{}", get_block_dev_dir(), part_name);
    let disk_path = get_partition_main_driver(&path)?;
    let sector_size = get_disk_sector_size(&disk_path);
    let mut sector = LogicalBlockSize::Lb512;
    if sector_size == 4096 {
        sector = LogicalBlockSize::Lb4096;
    } else if sector_size == 512 {} else {
        return Err("Error: unsupported sector size !!!");
    };
    let gptcfg = GptConfig::new().writable(false).logical_block_size(sector.clone());
    let mut disk = gptcfg.open(&disk_path);
    if !(disk.is_ok()) {
        return Err("Error: open disk failed");
    }
    let binding_disk = disk.unwrap();
    let (id, part) = binding_disk
        .partitions().iter()
        .find(|(_, partition)| partition.name == part_name)
        .ok_or("Error: partition not found")?;
    Ok((disk_path, *id, part.first_lba, part.last_lba, sector_size))
}

/// get disk sector size
/// panic if any error occurs
pub fn get_disk_sector_size(disk: &str) -> u64 {
    let path = Path::new(disk);
    let disk_name = path.file_name().unwrap().to_str().unwrap();
    let path = format!("/sys/class/block/{}/queue/logical_block_size", disk_name);
    let size_str = fs::read_to_string(path).expect("Error reading size file");
    let size: u64 = size_str.trim().parse().expect("Error parsing size");
    size
}

/// check if disk segment is used by table,return Option<Vec<(part_name,id)>>
/// panic if any error occurs
pub fn is_disk_segment_used(disk: &str, start_lba: u64, end_lba: u64) -> Option<Vec<(String, u32)>> {
    let sector_size = get_disk_sector_size(disk);
    let mut sector = LogicalBlockSize::Lb512;
    if sector_size == 4096 {
        sector = LogicalBlockSize::Lb4096;
    } else if sector_size == 512 {} else {
        panic!("Error: unsupported sector size !!!")
    };
    let gptcfg = GptConfig::new().writable(false).logical_block_size(sector.clone());
    let mut disk = gptcfg.open(disk).expect("open disk failed");
    let mut find_part_name = Vec::new();
    //check if given segment is used by some partitions,if any part of the segment is used,add it to the list
    for (id, partition) in disk.partitions().iter() {
        let interval_relat = check_interval_state(&Interval::new(start_lba, end_lba), &Interval::new(partition.first_lba, partition.last_lba));
        if interval_relat != IntervalState::Disjoint {
            find_part_name.push((partition.name.clone(), *id));
        }
    }
    if find_part_name.is_empty() {
        return None;
    };
    Some(find_part_name)
}

/// align partition lba (only shrink)
pub fn alignment_partition(first_lba: &mut u64, last_lba: &mut u64, alignment: u64, dis_align_last_lba: bool) {
    if *first_lba % alignment != 0 {
        *first_lba = *first_lba + alignment - *first_lba % alignment;
    }
    if dis_align_last_lba {
        if *last_lba % alignment == 0 {
            *last_lba = *last_lba - *last_lba % alignment - 1;
        }
    };
}

/// get disk partitions alignment via sysfs
/// panic if file read error or parse error , (rarely occurs)
pub fn get_disk_part_boundary_alignment(disk: &str) -> u32 {
    // cal via physical block size / logical block size
    let path = Path::new(disk);
    let disk_name = path.file_name().unwrap().to_str().unwrap();
    let path = format!("/sys/class/block/{}/queue/physical_block_size", disk_name);
    let size_str = fs::read_to_string(path).expect("Error reading size file");
    let phy_size: u32 = size_str.trim().parse().expect("Error parsing size");
    let path = format!("/sys/class/block/{}/queue/logical_block_size", disk_name);
    let size_str = fs::read_to_string(path).expect("Error reading size file");
    let log_size: u32 = size_str.trim().parse().expect("Error parsing size");
    phy_size / log_size
}