pub mod constants;
pub mod gpt_helper;
pub mod metadata;
pub mod android_flashable;
mod backup_factory;
mod backup_losetup;
mod backup_diskspace;
mod backup_partition;
mod backup_ftp;
mod config_helper;
mod math_support;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use gpt::disk::LogicalBlockSize;
use gpt::{GptConfig, partition};
use log::debug;
use constants::*;
use gpt_helper::get_userdata_driver;
use crate::backup_factory::BackupType;
use crate::gpt_helper::{auto_layout_freespace_example, bytes2ieee, calculate_firmware_size, delete_part_by_name, get_disk_sector_size, get_gpt_disk, get_part_accelerate_location, try_get_disk_lba};
use crate::math_support::Interval;
use crate::metadata::{Metadata, Slot, SlotsTomlConfig};

extern crate gpt;

/// Generate a template init config file
/// panic if any error occurs
pub fn generate_template_init_config_file(path: PathBuf, ex_back_fpath: Option<String>, dual_list: Option<String>) -> std::io::Result<()> {
    let userdata_driver = get_userdata_driver();
    let mut back_min_size_sector = 0;
    let sector_size = get_disk_sector_size(&userdata_driver);
    let mut sector = LogicalBlockSize::Lb512;
    if sector_size == 4096 {
        sector = LogicalBlockSize::Lb4096;
    } else if sector_size == 512 {} else {
        panic!("Error: unsupported sector size !!!")
    };
    let mut gpt_cfg = gpt::GptConfig::new().writable(false).logical_block_size(sector.clone());
    let mut disk_result = gpt_cfg.open(&userdata_driver);
    ///panic if disk open failed
    let mut disk = disk_result.expect("open disk failed");
    let mut userdata_id = 0;
    for (id, partition) in disk.partitions().iter() {
        if partition.name == USERDATA_NAME {
            userdata_id = *id;
        }
    }
    if userdata_id == 0 {
        panic!("Error: no userdata partition found");
    };
    disk.remove_partition(userdata_id).expect("remove partition failed");
    //find the largest free space
    let (start_lba, end_lba) = gpt_helper::find_max_free_tuple(&disk.find_free_sectors());
    println!("Find largest available space from {}*{} to {}*{} byte", start_lba, sector.as_u64(), end_lba, sector.as_u64());

    let (slot1, slot2) = auto_layout_freespace_example(&userdata_driver, start_lba, end_lba, sector.as_u64(), &ex_back_fpath, &dual_list, &mut back_min_size_sector);
    let config = SlotsTomlConfig { slot: vec![slot1, slot2] };
    let toml = toml::to_string(&config).unwrap();

    let mut file = std::fs::File::create(path)?;
    file.write_all(b"# DO NOT MODIFY BACKUP TARGET MIN SIZE , IT IS ONLY A INFO FOR YOU WITHOUT ANY OTHER FUNCTION\n");
    file.write_all(b"# DO NOT RESTORE SUPER IF ON SYSTEM\n");
    file.write_all(b"# Firmware too large ? add your custorm partitions to backup exclude list and  go on to check step \n");
    file.write_all(b"# On-system switch please make super or system and vender .etc to dual \n");
    file.write_all(b"# On-recovery switch is safe if umounted everything \n\n");
    let backup_target_min_size_note = format!("# NOTE : Backup Target Min SIZE = {} bytes , {}\n",
                                              back_min_size_sector * sector.as_u64(),
                                              bytes2ieee(back_min_size_sector * sector.as_u64()));
    file.write_all(backup_target_min_size_note.as_bytes());

    file.write_all(b"\n# Example config file\n\n");
    file.write_all(toml.as_bytes())
}

/// Check slots config file without really modify
pub fn check_slots_config(path: &str) {
    // check the flowing things (there are 1-several slots : 1 is fw size larger than backup size
    // 2 is any part overlaps and is anything overflows disk size
    let data = fs::read_to_string(path).expect("Error: read config file failed");
    let slots_config: SlotsTomlConfig = toml::from_str(&data).expect("Error: parse config file failed");
    let slots = &slots_config.slot;
    for slot in slots.iter() {
        println!("Checking for slot : {}", &slot.slot_name);
        // 1 first check fw size
        //merge exclude and dynamic partitions list into a total exclude list
        let mut exclude_list = HashSet::new();
        exclude_list.extend(slot.backup_exclude_list.iter().cloned());
        for (name, raw_pt) in slot.dyn_partition_set.iter() {
            exclude_list.insert(name.clone());
        }
        //calculate fw size
        let (num, size_bytes) = calculate_firmware_size(&exclude_list);
        let backup_target_size = (slot.backup_target_end - slot.backup_target_start + 1) * get_disk_sector_size(&slot.backup_target);
        if size_bytes > backup_target_size {
            println!("\t1 Error: firmware size {} bytes is larger than backup target size {} bytes , {} > {}",
                     size_bytes, backup_target_size, bytes2ieee(size_bytes), bytes2ieee(backup_target_size));
        } else {
            println!("\t1 Pass: firmware size {} bytes is smaller than backup target size {} bytes , {} < {}",
                     size_bytes, backup_target_size, bytes2ieee(size_bytes), bytes2ieee(backup_target_size));
        }

        // 2 check overlaps and is anything overflows disk size
        let ret = try_init_partition_table_layout(&path.to_string(), &Some(slot.slot_name.clone()), false, true);
        if ret.is_err() {
            eprintln!("\t2 {}", ret.err().unwrap());
        } else {
            println!("\t2 Pass: no overlaps and no overflow disk size");
        }
        //TODO feather check
    }
    println!("Check done");
}

/// Try init partition table layout
/// this is the cache version of init_partition_table_layout
pub fn try_init_partition_table_layout(cfg_path: &String, initial_slot: &Option<String>, save_changes: bool, silent: bool) -> Result<(), &'static str> {
    //print silent warning if silent mode enabled
    if silent && save_changes {
        println!("Warning: silent mode enabled, allow all dangerous actions");
    }
    let data = fs::read_to_string(cfg_path).expect("Error: read config file failed");
    let slots_config: SlotsTomlConfig = toml::from_str(&data).expect("Error: parse config file failed");
    let slots = &slots_config.slot;
    let mut target_slot;
    if let Some(init_target) = initial_slot {
        target_slot = slots.iter().find(|&x| x.slot_name == *init_target).expect("Error: no such slot found");
    } else {
        target_slot = slots.get(0).expect("Error: no slot found");
    }
    // part tables backup , store orig part table in ram
    let mut tables_backup = HashMap::new();
    if save_changes {
        for (part_name, raw_part) in &target_slot.dyn_partition_set {
            // store orig part table in ram
            let disk_ret = get_gpt_disk(&raw_part.driver, false).expect("Error: get disk failed");
            tables_backup.insert(raw_part.driver.clone(), disk_ret);
            let (driver2, id, _, _, _) = get_part_accelerate_location(part_name)
                .expect("Error: get part accelerate location failed");
            let disk_ret = get_gpt_disk(&driver2, false).expect("Error: get disk failed");
            tables_backup.insert(driver2.clone(), disk_ret);
            debug!("Backup part table for part {} on disk {}", part_name, raw_part.driver);
        };
    }

    // move to the target slot
    let move_ret = init_partition_table_layout(target_slot, save_changes, silent);
    if move_ret.is_err() {
        // restore all changed tables
        println!("Error: init partition table layout failed, restoring all changed tables");
        for (driver, disk) in tables_backup {
            let ret=disk.write();
            if ret.is_err() {
                println!("Error: restore disk {} failed", driver);
            };
        }
        return Err("Error: init partition table layout failed");
    };
    Ok(())
}

/// init partition table layout
/// ## Never panics
pub fn init_partition_table_layout(target_slot: &Slot, save_changes: bool, silent: bool) -> Result<(), &'static str> {
    // move to the target slot
    for (part_name, raw_part) in &target_slot.dyn_partition_set {
        debug!("Init part table for part {} on {}", part_name, raw_part.driver);
        //delete part
        //TODO BIG fuck bug
        delete_part_by_name(part_name)?;
        //create part
        let disk = raw_part.driver.clone();
        let mut gptcfg = gpt::GptConfig::new().writable(true).logical_block_size(try_get_disk_lba(&disk)).change_partition_count(true);
        let mut disk = gptcfg.open(&disk).map_err(|_| "Error: open disk failed")?;
        let partition_id = disk
            .find_next_partition_id()
            .unwrap_or_else(|| {
                println!("Warning: no free partition id found, will increase partition number");
                if !silent {
                    //ask if process
                    let mut input = String::new();
                    if save_changes {
                        println!("Warning: We have cached all target gpt tables in ram");
                        println!("Enter n will restore all gpt table to original state");
                    };

                    println!("Do you want to continue ? (y/n)");
                    let ret=std::io::stdin().read_line(&mut input);
                    if ret.is_err() {
                        println!("Std Error: read input failed, auto enter n");
                        return 0;
                    };
                    if input.trim() != "y" || input.trim() != "Y" {
                        // give an impossible id to stop process
                        return 0;
                    };
                };
                disk.header().num_parts + 1
            });
        if partition_id == 0 {
            return Err("Error: user cancel process");
        };
        let orig_part_num = disk.header().num_parts;
        let mut partitions = disk.take_partitions();
        let part_type = gpt::partition_types::Type::from_name(&raw_part.type_guid.clone()).map_err(|_| "Error: invalid part type guid")?;
        let part = partition::Partition {
            part_type_guid: part_type,
            part_guid: uuid::Uuid::new_v4(),
            first_lba: raw_part.start_lba,
            last_lba: raw_part.end_lba,
            flags: raw_part.flags,
            name: part_name.to_string(),
        };
        partitions.insert(partition_id, part);
        disk.update_partitions(partitions).map_err(|_| "Error: update partitions failed")?;
        let new_part_num = disk.header().num_parts;
        if new_part_num != orig_part_num {
            println!("Warning: partition limits changed from {} to {}", orig_part_num, new_part_num);
        };
        if save_changes {
            disk.write().map_err(|_| "Error: write disk failed")?;
        };
    };
    Ok(())
}


/// update config to all slots
pub fn update_config_to_all_slots(path: &str) {
    let data = fs::read_to_string(path).expect("Error: read config file failed");
    let slots_config: SlotsTomlConfig = toml::from_str(&data).expect("Error: parse config file failed");
    let slots = slots_config.slot;
    if cfg!(debug_assertions) {
        println!("Debug: update config to all slots {:?}", slots);
    };
    //vec to hashmap
    let mut slots_map: HashMap<String, Slot> = HashMap::new();
    for slot in slots {
        slots_map.insert(slot.slot_name.clone(), slot);
    }
    let mut metadata = Metadata::new("unknown".to_string(), slots_map);
    metadata.calculate_current_slot();
    metadata.write_fw_metadata().unwrap();
}

/// show current slot and its metadata
pub fn show_current_slot(only_name: bool) {
    let metadata = Metadata::from_fw_metadata().unwrap();
    let slot = metadata.slots.get(metadata.current_slot.as_str()).expect("Error: current slot match no item");
    println!("Current Slot : {}", metadata.current_slot);
    if only_name {
        return;
    }
    //print current slot metadata
    println!("Slot Name : {:?}", slot);
}


/// list slots (slot)
pub fn list_slots(slot_name: Option<String>, only_name: bool) {
    let metadata = Metadata::from_fw_metadata().unwrap();
    if let Some(slot_name) = slot_name {
        let slot = metadata.slots.get(slot_name.as_str()).expect("Error: no such slot found");
        println!("{}", slot);
        return;
    }

    if only_name {
        let mut counter = 0;
        for (slot_name, _) in metadata.slots.iter() {
            counter += 1;
            println!("{} : {}", counter, slot_name);
        };
        return;
    }
    println!("{}", metadata);
}

/// dump current metadata to a config file
pub fn dump_current_metadata(path: &str) {
    let metadata = Metadata::from_fw_metadata().unwrap();
    //convert slot toml
    let slots_config = SlotsTomlConfig { slot: metadata.slots.values().cloned().collect() };
    let toml = toml::to_string(&slots_config).unwrap();
    let mut file = std::fs::File::create(path).expect("Error: create dump file failed");
    file.write_all(toml.as_bytes()).expect("Error: write dump file failed");
}