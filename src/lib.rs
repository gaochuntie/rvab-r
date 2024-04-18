mod constants;
mod gpt_helper;
mod metadata;
mod android_flashable;
mod backup_factory;
mod backup_losetup;
mod backup_diskspace;
mod backup_partition;
mod backup_ftp;
mod config_helper;
mod math_support;

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use gpt::disk::LogicalBlockSize;
use constants::*;
use gpt_helper::get_userdata_driver;
use crate::backup_factory::BackupType;
use crate::gpt_helper::{auto_layout_freespace_example, bytes2ieee, get_disk_sector_size};
use crate::metadata::{Metadata, Slot, SlotsTomlConfig};

extern crate gpt;

/// Generate a template init config file
/// panic if any error occurs
pub fn generate_template_init_config_file(path: PathBuf, ex_back_fpath: Option<String>, dual_list: Option<String>) -> std::io::Result<()> {
    let userdata1_driver = get_userdata_driver();
    let mut back_min_size_sector = 0;
    let sector_size = get_disk_sector_size(&userdata1_driver);
    let mut sector = LogicalBlockSize::Lb512;
    if sector_size == 4096 {
        sector = LogicalBlockSize::Lb4096;
    } else if sector_size == 512 {} else {
        panic!("Error: unsupported sector size !!!")
    };

    let mut gpt_cfg = gpt::GptConfig::new().writable(false).logical_block_size(sector.clone());
    let mut disk_result = gpt_cfg.open(&userdata1_driver);
    ///panic if disk open failed
    let mut disk = disk_result.expect("open disk failed");
    let mut userdata_id = 0;
    for (id, partition) in disk.partitions().iter() {
        if partition.name == USERDATA_NAME {
            userdata_id = *id;
        }
    }
    disk.remove_partition(userdata_id).expect("remove partition failed");
    //find largest free space
    let (start_lba, end_lba) = gpt_helper::find_max_free_tuple(&disk.find_free_sectors());
    println!("Find largest available space from {}*{} to {}*{} byte", start_lba, sector.as_u64(), end_lba, sector.as_u64());

    let (slot1, slot2) = auto_layout_freespace_example(start_lba, end_lba, sector.as_u64(), &ex_back_fpath, &dual_list, &mut back_min_size_sector);
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
    // check the flowing things (there are 1-several slots : 1 is fw size larger than backup size 2 is any part overlaps 3 is any thing overflows disk size
    let data=fs::read_to_string(path).expect("Error: read config file failed");
    let slots_config: SlotsTomlConfig = toml::from_str(&data).expect("Error: parse config file failed");
    let slots = slots_config.slot;
    for slot in slots.iter(){
        let fw_size=
    }
}

/// Init partition table layout
pub fn init_partition_table_layout(path: &str) {}

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
        let mut counter=0;
        for (slot_name, _) in metadata.slots.iter() {
            counter += 1;
            println!("{} : {}",counter, slot_name);
        };
        return;
    }
    println!("{}", metadata);
}

///
pub fn dump_current_metadata(path:&str) {
    let metadata = Metadata::from_fw_metadata().unwrap();
    //convert slot toml
    let slots_config=SlotsTomlConfig{slot:metadata.slots.values().cloned().collect()};
    let toml = toml::to_string(&slots_config).unwrap();
    let mut file = std::fs::File::create(path).expect("Error: create dump file failed");
    file.write_all(toml.as_bytes()).expect("Error: write dump file failed");
}