use std::fmt;
use serde::{Deserialize, Serialize};
use crate::metadata::Metadata;

/// BackupTrait is a trait for backup and restore object between original disk segement and unknown target
/// The target could be a file, a partition, a disk, a network, a cloud, etc.
/// Inner implementation ways include partition,binary space disk segement,losetup partition)
pub trait BackupTrait {
    fn backup(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str>;
    fn restore(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str>;
    //backup gpt table
    fn backup_gpt(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str>;
    //restore gpt table
    fn restore_gpt(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str>;
    //verify backup
    fn verify(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str>;
    fn test(&self,metadata: Option<&Metadata>) -> Result<(),&str>;
}
#[derive(Debug,Serialize,Deserialize,Copy, Clone)]
pub enum BackupType {
    Partition,
    BinarySpace,
    Losetup,
}
impl fmt::Display for BackupType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            BackupType::Partition => "partition",
            BackupType::BinarySpace => "binaryspace",
            BackupType::Losetup => "losetup",
        })
    }
}

impl BackupType {
    /// Convert a code number to a BackupType,typically from a config file
    pub fn code2type(code: i32) -> Result<BackupType, &'static str> {
        match code {
            0 => Ok(BackupType::Partition),
            1 => Ok(BackupType::Losetup),
            2 => Ok(BackupType::BinarySpace),
            _ => Err("Invalid BackupType code"),
        }
    }
    /// Convert a BackupType to a code number,typically for a config file
    pub fn type2code(backup_type: BackupType) -> i32 {
        match backup_type {
            BackupType::Partition => 0,
            BackupType::Losetup => 1,
            BackupType::BinarySpace => 2,
        }
    }
    /// Guess the backup target partition size (bytes)
    /// over 10% of the firmware size
    pub fn guess_backup_target_partition_size(&self,firmware_size_b: u64) -> u64 {
        match self {
            BackupType::Partition => {
                firmware_size_b*(1.1) as u64
            }
            BackupType::BinarySpace => {
                firmware_size_b*(1.1) as u64
            }
            BackupType::Losetup => {
                firmware_size_b*(1.1) as u64
            }
        }
    }

    pub fn guess_backup_target_partition_size_sector(&self,firmware_size_b: u64,sector:u64) -> u64 {
        let size = self.guess_backup_target_partition_size(firmware_size_b);
        let mut size_sector=size/sector;
        if size%sector!=0 {
            size_sector+=1;
        }
        size_sector
    }
}

impl BackupTrait for BackupType {
    fn backup(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str> {
        Ok(())
    }
    fn restore(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str> {
        Ok(())
    }
    fn backup_gpt(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str> {
        Ok(())
    }
    fn restore_gpt(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str> {
        Ok(())
    }
    fn verify(&self,metadata: &Metadata,orig_file_path: &str) -> Result<(),&str> {
        Ok(())
    }
    fn test(&self,metadata: Option<&Metadata>) -> Result<(),&str> {
        match self {
            BackupType::Partition => {}
            BackupType::BinarySpace => {}
            BackupType::Losetup => {}
        }
        Ok(())
    }

}


