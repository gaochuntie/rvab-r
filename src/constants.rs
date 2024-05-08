use std::fs;

// pub const CONFIG_INIT_SLOT_NAME: &str = "slot_name";
// pub const CONFIG_INIT_BACKUP_TYPE: &str = "backup_type";
// pub const CONFIG_INIT_USERDATA_DRIVER: &str = "userdata_driver";
// pub const CONFIG_INIT_USERDATA_START: &str = "userdata_start";
// pub const CONFIG_INIT_USERDATA_END: &str = "userdata_end";
// pub const CONFIG_INIT_BACKUP_TARGET: &str = "backup_target";
// pub const CONFIG_INIT_BACKUP_TARGET_START: &str = "backup_target_start";
// pub const CONFIG_INIT_BACKUP_TARGET_END: &str = "backup_target_end";
// pub const CONFIG_INIT_BACKUP_TARGET_ATTR: &str = "backup_target_attr";
// pub const CONFIG_INIT_BACKUP_TARGET_MIN_SIZE: &str = "backup_target_min_size";
// pub const CONFIG_INIT_WARNING: &str = "Warning";

pub const EMMC_TRAIT_FILE: &str = "/dev/block/mmcblk0";
pub const USERDATA_NAME: &str = "userdata";
pub const USERDATA_LINK_PATH: &str = "/dev/block/by-name/userdata";
pub const USERDATA_LINK_PATH_BOOT: &str = "/dev/block/bootdevice/by-name/userdata";
pub const USERDATA_LINK_PATH_PLATFORM: &str = "/dev/block/platform/soc/*/by-name/userdata";
/// 5G min size for userdata
pub const USERDATA_MIN_SIZE: u64 = 1024 * 1024 * 1024 * 5;

pub const BLOCK_DEV_NAME_MAPPER: &str = "/dev/block/by-name/";
pub const BLOCK_DEV_NAME_BOOT: &str = "/dev/block/bootdevice/by-name/";
pub const BLOCK_DEV_NAME_PLATFORM: &str = "/dev/block/platform/soc/*/by-name/";
pub const BLOCK_DEV_DIR: &str = "/dev/block/";

pub const BACK_EXCLUDE_LIST: [&'static str; 1] = ["userdata"];
pub const METADATA_PARTITION_NAME: &str = "rvab_metadata";
pub const METADATA_HEAD_MAGIC: &'static str = "RVAB_HEAD_MAGIC";
pub const METADATA_TAIL_MAGIC: &'static str = "RVAB_TAIL_MAGIC";

/// Get the block device name mapper dir path
/// panic if not found
pub fn get_block_dev_dir() -> String {
    let block_dev_dir = BLOCK_DEV_NAME_MAPPER;
    if fs::metadata(block_dev_dir).is_ok() {
        return block_dev_dir.to_string();
    }
    let block_dev_dir = BLOCK_DEV_NAME_BOOT;
    if fs::metadata(block_dev_dir).is_ok() {
        return block_dev_dir.to_string();
    }
    let block_dev_dir = BLOCK_DEV_NAME_PLATFORM;
    if fs::metadata(block_dev_dir).is_ok() {
        return block_dev_dir.to_string();
    }
    panic!("Error: android block dev name mapper dir not found");
}
