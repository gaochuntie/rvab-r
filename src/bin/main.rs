use std::fs;
use std::thread::current;
use argh::FromArgs;
use librvab_cli_r::{check_slots_config, dump_current_metadata, generate_template_init_config_file, try_init_partition_table_layout, list_slots, show_current_slot, update_config_to_all_slots};

#[derive(FromArgs)]
/// rvab command line multi call tool,
/// manage <real> virtual A/B/C/D... slots for Android devices
/// yes,these slots is not those slots
struct CmdProg {
    #[argh(subcommand)]
    /// subcommand
    mode: Mode,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Mode {
    Init(InitMode),
    Install(InstallMode),
    Switch(SwitchMode),
    List(ListMode),
    Current(Current),
    Archive(ArchiveMode),
}

#[derive(FromArgs)]
#[argh(subcommand,
name = "init",
description = "set necessary gpt layout: move and resize userdata,\
blank 64mib before each userdata partition,and free up space for backup and other slots. \
Use -o <output file> to generate a template config file",
example = "rvab init -f <config> ",
example = "rvab init -f <config> --slot a",
)]
/// set necessary gpt layout
struct InitMode {
    /// config file for init
    #[argh(option, short = 'f')]
    config: Option<String>,
    /// which slot do you want to be the first slot , default is the first slot in config file
    #[argh(option)]
    slot: Option<String>,
    /// generate template config file
    #[argh(option, short = 't')]
    template: Option<String>,
    /// exclude-list file path for backup partition name
    #[argh(option)]
    exclude: Option<String>,
    /// dyn partitions list (userdata auto included) , will be added to exclude list automatically
    #[argh(option)]
    dynpt: Option<String>,
    /// check and test config file without modify disk
    #[argh(option, short = 'c')]
    check: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand,
name = "install",
description = "install(update) metadata for all slots (all slots' metadata must keep synced)",
example = "rvab install -u <config>",
example = "rvab install -t <output template file> --exclude <exclude list file> --dynpt <dyn partitions list file>",
example = "rvab install -c <config>",
)]
/// install(update) metadata for all slots
struct InstallMode {
    /// update config for all slot metadata
    #[argh(option, short = 'u')]
    update: Option<String>,
    /// generate template config file
    #[argh(option, short = 't')]
    template: Option<String>,
    /// exclude-list file path for backup partition name
    #[argh(option)]
    exclude: Option<String>,
    /// dual partitions list (userdata auto included) , will be added to exclude list automatically
    #[argh(option)]
    dynpt: Option<String>,
    /// check and test config file without modify disk
    #[argh(option, short = 'c')]
    check: Option<String>,
    /// dump current metadata to file
    #[argh(option, short = 'd')]
    dump: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "switch")]
/// switch to another slot
struct SwitchMode {
    /// slot name (max 16 length ascii string)
    #[argh(positional)]
    slot: String,
}

#[derive(FromArgs)]
#[argh(subcommand,
name = "list",
example = "rvab list [slot]")]
/// list all slots and metadata
struct ListMode {
    /// target slot
    #[argh(positional)]
    slot: Option<String>,
    /// only show current slot name
    #[argh(switch, short = 'n')]
    name: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "current")]
/// show current slot and metadata
struct Current {
    /// only show current slot name
    #[argh(switch, short = 'n')]
    name: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "archive")]
/// archive certain slot to a recovery flashable zip file
struct ArchiveMode {
    /// archive slot name
    #[argh(positional)]
    slot: String,
    /// output file
    #[argh(option, short = 'o')]
    output: String,
    /// enable backup full gpt table
    #[argh(switch, short = 'g')]
    gpt: bool,
}

fn main() {
    let args: CmdProg = argh::from_env();
    match args.mode {
        Mode::Init(init) => {
            println!("Init mode");
            if let Some(out) = init.template {
                let path = std::path::PathBuf::from(out);
                generate_template_init_config_file(path,init.exclude,init.dynpt).unwrap();
                return;
            }
            if let Some(check) = init.check {
                check_slots_config(&check);
                return;
            }
            if let Some(config) = init.config {
                let ret = try_init_partition_table_layout(&config, &init.slot, true);
                if ret.is_err() {
                    eprintln!("Init failed {}", ret.err().unwrap());
                    //,try restoring gpt partition table...
                }
                return;
            }
            println!("Option required");
            return;
        }
        Mode::Install(install) => {
            println!("Install mode");
            if let Some(out) = install.template {
                let path = std::path::PathBuf::from(out);
                generate_template_init_config_file(path,install.exclude,install.dynpt).unwrap();
                return;
            }
            if let Some(check) = install.check {
                check_slots_config(&check);
                return;
            }
            if let Some(config) = install.update {
                update_config_to_all_slots(&config);
                return;
            }
            if let Some(dump_file) = install.dump {
                dump_current_metadata(&dump_file);
                return;
            };
            println!("Option required");
            return;
        }
        Mode::Switch(_) => {
            println!("Switch mode");
        }
        Mode::List(list) => {
            println!("List mode");
            list_slots(list.slot, list.name);
        }
        Mode::Current(current) => {
            println!("Current mode");
            show_current_slot(current.name);
        }
        Mode::Archive(_) => {
            println!("Archive mode");
        }
    }
}

