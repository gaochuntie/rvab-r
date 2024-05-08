/// from android-13-r43
/******************************************************************************
 * AB RELATED DEFINES
 ******************************************************************************/
// Bit 48 onwards in the attribute field are the ones where we are allowed to
// store our AB attributes.
const AB_FLAG_OFFSET: usize = 6;
const GPT_DISK_INIT_MAGIC: u32 = 0xABCD;
const AB_PARTITION_ATTR_SLOT_ACTIVE: u8 = 0x1 << 2;
const AB_PARTITION_ATTR_BOOT_SUCCESSFUL: u8 = 0x1 << 6;
const AB_PARTITION_ATTR_UNBOOTABLE: u8 = 0x1 << 7;
const AB_SLOT_ACTIVE_VAL: u8 = 0xF;
const AB_SLOT_INACTIVE_VAL: u8 = 0x0;
const AB_SLOT_ACTIVE: u8 = 1;
const AB_SLOT_INACTIVE: u8 = 0;
const AB_SLOT_A_SUFFIX: &str = "_a";
const AB_SLOT_B_SUFFIX: &str = "_b";
const PTN_XBL: &str = "xbl";
const PTN_SWAP_LIST: [&str; 14] = [
    PTN_XBL,
    "abl",
    "aop",
    "devcfg",
    "dtbo",
    "hyp",
    "keymaster",
    "qupfw",
    "tz",
    "uefisecapp",
    "vbmeta",
    "vbmeta_system",
    "xbl_config",
    "featenabler",
];
const AB_PTN_LIST: [&str; 21] = [
    PTN_XBL,
    "abl",
    "aop",
    "devcfg",
    "dtbo",
    "hyp",
    "keymaster",
    "qupfw",
    "tz",
    "uefisecapp",
    "vbmeta",
    "vbmeta_system",
    "xbl_config",
    "featenabler",
    "boot",
    "vendor_boot",
    "system",
    "vendor",
    "system_ext",
    "modem",
    "product",
];
const BOOT_DEV_DIR: &str = "/dev/block/bootdevice/by-name";

/// Get the current active android slot
pub fn get_current_android_slot() -> String {
    // TODO
    "a".to_string()
}
pub fn change_active_android_slot(slot: &str) {
    // TODO
}
