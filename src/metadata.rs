///metadata module
use serde::{Deserialize, Serialize};

//64MIB
pub const METADATA_SIZE_BYTES: u64 = 1024 * 1024 * 64;

use std::collections::{HashMap, HashSet};
use std::f32::consts::E;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::os::unix::fs::FileExt;
use crc32fast::Hasher;
use toml::to_string as toml_to_string;
use std::string::String as std_string;
use crate::backup_factory::BackupType;
use crate::constants::{get_block_dev_dir, METADATA_HEAD_MAGIC, METADATA_PARTITION_NAME, METADATA_TAIL_MAGIC, USERDATA_NAME};
use crate::gpt_helper::{get_disk_sector_size, get_part_accelerate_location, get_userdata_driver, is_disk_segment_used};

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    //on disk,this field is always false to avoid crc32 mess
    is_dirty: bool,
    pub current_slot: String,
    pub slots: HashMap<String, Slot>,
}
impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Is Dirty: {}\nCurrent Slot: {}\n\n", self.is_dirty, self.current_slot)?;
        for (_, slot) in &self.slots {
            write!(f, "{}\n", slot)?;
        }
        Ok(())
    }
}
impl Metadata {
    pub fn new(current_slot: String, slots: HashMap<String, Slot>) -> Self {
        Metadata {
            is_dirty: false,
            current_slot,
            slots,
        }
    }
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }
    pub fn from_toml(path: &str) -> Result<Self, &'static str> {
        let toml_str = std::fs::read_to_string(path);
        if toml_str.is_ok() {
            let ret = toml::from_str(&toml_str.unwrap());
            if ret.is_ok() {
                ret.unwrap()
            }
            Err("Unable to parse toml file")
        } else {
            Err("Unable to read toml file")
        }
    }

    /// lazy match current slot (just match userdata)
    pub fn calculate_current_slot(&mut self) -> Option<String> {
        let mut current_slot = String::new();
        let (driver, _, first_lba, last_lba, sector_size) = get_part_accelerate_location(USERDATA_NAME).unwrap_or(("".to_string(), 0, 0, 0, 0));
        if driver.is_empty() {
            return None;
        };
        for (slot_name, slot) in self.slots.iter() {
            let userdata_target = &slot.dyn_partition_set.get(USERDATA_NAME);
            if userdata_target.is_none() {
                continue;
            };
            let userdata_target = userdata_target.unwrap();
            if userdata_target.driver == driver && userdata_target.start_lba == first_lba && userdata_target.end_lba == last_lba {
                current_slot = slot_name.clone();
                self.current_slot = current_slot.clone();
                return Some(current_slot);
            }
        }
        self.current_slot = "unknown".to_string();
        None
    }
    /// read metadata from fw metadata segment
    /// panic if crc32 not match and give N option
    pub fn from_fw_metadata() -> Result<Self, &'static str> {
        let (main_driver, id, start_lba, end_lba, sector_size) = Metadata::get_current_metadata()?;
        let offset = start_lba * sector_size;

        //tail reserved 4 bytes for crc32
        let length = (end_lba - start_lba + 1) * sector_size - 4;
        // read data from main_driver using offset and limited max length,and read crc32 from tail
        //the actrual data is between METADATA_HEAD_MAGIC and METADATA_TAIL_MAGIC
        let mut toml_str = String::new();
        let crc32 = 0;
        let mut count: u64 = 0;

        //warning : this field on disk is always false to avoid crc32 mess
        let mut is_dirty = false;

        //check head magic
        let mut magic_head_buffer = [0; METADATA_HEAD_MAGIC.as_bytes().len()];
        let mut file = File::open(main_driver).map_err(|_| "Error: Failed to open metadata partition")?;
        file.read_exact_at(&mut magic_head_buffer, offset).map_err(|_| "Error: Failed to read header")?;
        if magic_head_buffer != METADATA_HEAD_MAGIC.as_bytes() {
            return Err("Error: metadata head magic not match");
        };
        count+=magic_head_buffer.len() as u64;
        let mut reader = BufReader::new(&file);
        reader.seek(SeekFrom::Start(offset + (magic_head_buffer.len() as u64))).map_err(|_| "Error: Failed to seek")?;
        for line in reader.lines() {
            if line.is_ok() {
                let lin_str = line.unwrap();
                count += lin_str.as_bytes().len() as u64 + 1;
                if (count > length) || (lin_str == METADATA_TAIL_MAGIC) {
                    break;
                };
                toml_str.push_str(&lin_str);
                toml_str.push('\n');
            } else {
                break;
            }
        };
        //read crc32
        let mut crc32_buffer = [0; 4];
        file.read_exact_at(&mut crc32_buffer, offset + length).map_err(|_| "Error: Failed to read crc32")?;
        //get crc32
        let crc32 = u32::from_le_bytes(crc32_buffer);
        //calculate string crc32
        let mut hasher = Hasher::new();
        hasher.update(toml_str.as_bytes());
        let checksum = hasher.finalize();
        if checksum != crc32 {
            is_dirty = true;
            println!("Warning: metadata crc32 not match,maybe metadata is dirty.\n\
            Please update metadata using rvab instead of modify it manually\n");
            //ask if continue
            println!("Force use dirty metadata ? (Y/N) ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).expect("Error: Failed to read line");
            if input.trim().to_uppercase() != "Y" {
                panic!("Exit ...")
            };

        };
        //parse toml
        let mut metadata: Metadata = toml::from_str(&toml_str).map_err(|_| "Failed to parse toml")?;
        metadata.is_dirty = is_dirty;
        metadata.calculate_current_slot();
        Ok(metadata)
    }
    pub fn write_fw_metadata(&mut self) -> Result<(), &'static str> {
        let metadata = Metadata::get_all_metadata_location(&self)?;

        //warning : this field on disk is always false to avoid crc32 mess
        let is_dirty_backup = self.is_dirty;
        self.is_dirty = false;
        let mut toml_str = toml_to_string(&self).map_err(|_| "Error: Failed to convert to toml str")?;
        //add \n to avoid read overflow
        toml_str.push('\n');

        //cal crc32
        let mut hasher = Hasher::new();
        hasher.update(toml_str.as_bytes());
        let checksum = hasher.finalize();
        let crc32_buffer = checksum.to_le_bytes();


        toml_str.push_str(METADATA_TAIL_MAGIC);
        toml_str.push('\n');
        //restore is_dirty
        self.is_dirty = is_dirty_backup;

        for (main_driver, _, start_lba, end_lba, sector_size) in metadata {
            if cfg!(debug_assertions) {
                println!("Main driver: {}", main_driver);
                println!("Start LBA: {}", start_lba);
                println!("End LBA: {}", end_lba);
                println!("Sector size: {}", sector_size);
            }
            let offset = start_lba * sector_size;

            //tail reserved 4 bytes for crc32
            let max_length = (end_lba - start_lba + 1) * sector_size - 4;


            //write head magic
            let magic_head_buffer = METADATA_HEAD_MAGIC.as_bytes();
            let mut file = OpenOptions::new().write(true).open(main_driver).map_err(|_| "Error: Failed to open metadata partition")?;
            file.write_all_at(&magic_head_buffer, offset).map_err(|_| "Error: Failed to write header")?;

            if (toml_str.as_str().len() + magic_head_buffer.len()) as u64 > max_length {
                return Err("Error: metadata size overflow");
            };
            file.write_all_at(toml_str.as_bytes(), offset + (magic_head_buffer.len() as u64)).map_err(|_| "Error: Failed to write to metadata partition")?;

            file.write_all_at(&crc32_buffer, offset + max_length).map_err(|_| "Error: Failed to write crc32");
        };
        Ok(())
    }

    /// search and check all metadata partition ret vec (main_driver,id,start_lba,end_lba,sector_size)
    /// search priority: name mapped block device -> hidden segment
    /// id = 0 means hidden segment
    pub fn get_all_metadata_location(metadata: &Metadata) -> Result<Vec<(String, u32, u64, u64, u64)>, &'static str> {
        let mut ret = Vec::new();
        //first test name mapped block device
        let name_ret = get_part_accelerate_location(METADATA_PARTITION_NAME);
        if name_ret.is_ok() {
            let (main_driver, id, start_lba, end_lba, sector_size) = name_ret.unwrap();
            ret.push((main_driver.clone(), id, start_lba, end_lba, sector_size));
            return Ok(ret);
        };

        for (_, slot) in metadata.slots.iter() {
            let userdata_target = slot.dyn_partition_set.get(USERDATA_NAME);
            if userdata_target.is_none() {
                return Err("Error: userdata partition not found");
            };
            let userdata_target = userdata_target.unwrap();
            let userdata_driver = userdata_target.driver.clone();
            let userdata_start_lba = userdata_target.start_lba;
            let sector_size = get_disk_sector_size(&userdata_driver);
            //check hidden segment
            let (metadata_start_lba, metadata_end_lba) = calculate_metadata_interval_from_low(userdata_start_lba - 1, sector_size);
            //check overflows
            let overflows = is_disk_segment_used(&userdata_driver, metadata_start_lba, metadata_end_lba);
            if overflows.is_some() {
                let error_str = format!("Error: metadata partition overflows at {:?}", overflows.unwrap());
                eprintln!("{}", error_str.as_str());
                return Err("Error: metadata partition overflows");
            };
            ret.push((userdata_driver, 0, metadata_start_lba, metadata_end_lba, sector_size));
        };
        Ok(ret)
    }

    ///get current metadata
    pub fn get_current_metadata() -> Result<(String, u32, u64, u64, u64), &'static str> {
        let userdata_location = get_part_accelerate_location(USERDATA_NAME);
        if userdata_location.is_err() {
            return Err("Error: userdata partition not found");
        };
        let (userdata_driver, _, userdata_start_lba, _, sector_size) = userdata_location.unwrap();
        //check hidden segment
        let (metadata_start_lba, metadata_end_lba) = calculate_metadata_interval_from_low(userdata_start_lba - 1, sector_size);
        //check overflows
        let overflows = is_disk_segment_used(&userdata_driver, metadata_start_lba, metadata_end_lba);
        if overflows.is_some() {
            let error_str = format!("Error: metadata partition overflows at {:?}", overflows.unwrap());
            eprintln!("{}", error_str.as_str());
            return Err("Error: metadata partition overflows");
        };
        Ok((userdata_driver, 0, metadata_start_lba, metadata_end_lba, sector_size))
    }
}

#[derive(Debug,Clone, Serialize, Deserialize)]
pub struct Slot {
    pub slot_name: String,
    pub backup_type_code: i32,
    // pub userdata_driver: String,
    // pub userdata_start_lba: u64,
    // pub userdata_end_lba: u64,
    pub backup_target: String,
    pub backup_exclude_list: HashSet<String>,
    pub backup_target_start: u64,
    pub backup_target_end: u64,
    pub backup_target_attr: String,
    //reserve for other back_trait
    pub dyn_partition_set: HashMap<String, PartitionRawTarget>,// parts to be made into dyn partition,(part_name,part_target)
}
impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Slot Name: {}\nBackup Type Code: {}\nBackup Target: {}\nBackup Target Start: {}\nBackup Target End: {}\nBackup Target Attr: {}\n",
               self.slot_name, self.backup_type_code, self.backup_target, self.backup_target_start, self.backup_target_end, self.backup_target_attr)?;

        write!(f, "Backup Exclude List:\n")?;
        let mut counter = 0;
        for exclude in &self.backup_exclude_list {
            counter+=1;
            write!(f, "\t{} : {}\n", counter,exclude)?;
        };
        write!(f, "Dynamic Partition Set:\n")?;
        counter=0;
        for (part_name, part_target) in &self.dyn_partition_set {
            counter+=1;
            write!(f, "\t-- {counter} --\n\tPartition Name: {}\n\tDriver: {}\n\tStart LBA: {}\n\tEnd LBA: {}\n\n",
                   part_name, part_target.driver, part_target.start_lba, part_target.end_lba)?;
        }
        Ok(())
    }
}
#[derive(Debug,Clone, Serialize, Deserialize)]
pub struct PartitionRawTarget {
    pub part_name: String,
    pub driver: String,
    pub start_lba: u64,
    pub end_lba: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlotsTomlConfig {
    pub slot: Vec<Slot>,
}

/// Calculate the metadata interval ,return (start_lba,end_lba)
pub fn calculate_metadata_interval(start_lba: u64, sector: u64) -> (u64, u64) {
    let mut end_lba = start_lba + METADATA_SIZE_BYTES / sector;
    if METADATA_SIZE_BYTES % sector != 0 {
        end_lba += 1;
    }
    //tail head include
    end_lba -= 1;
    (start_lba, end_lba)
}

/// Calculate the metadata interval from low (inclusive) ,return (start_lba,end_lba)
pub fn calculate_metadata_interval_from_low(end_lba: u64, sector: u64) -> (u64, u64) {
    let mut start_lba = end_lba - METADATA_SIZE_BYTES / sector;
    if METADATA_SIZE_BYTES % sector != 0 {
        start_lba -= 1;
    }
    //tail head include
    start_lba += 1;
    (start_lba, end_lba)
}

