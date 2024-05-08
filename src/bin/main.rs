use argh::FromArgs;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use librvab_cli_r::{
    check_slots_config, dump_current_metadata, generate_template_init_config_file, list_slots,
    show_current_slot, try_init_partition_table_layout, try_init_userdata_partition,
    update_config_to_all_slots,
};
use rand::Rng;
use std::cmp::min;
use std::fmt::Write;
use std::thread::current;
use std::time::Duration;
use std::{fs, thread};

#[derive(FromArgs)]
/// rvab command line multi call tool,
/// manage <real> virtual A/B/C/D... slots for Android devices
/// yes,these slots is not those slots
struct CmdProg {
    #[argh(switch, short = 's')]
    /// silent mode, allow all dangerous actions
    silent: bool,
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
    Test(TestMode),
}

#[derive(FromArgs)]
#[argh(
    subcommand,
    name = "init",
    description = "Use -o <output file> to generate a template config file. \
Use -c <config> to check and test config file without modify disk. \
Use -f <config> to init userdata partition then. \
Use -full <config> to init and sync(clone) all dyn partitions except userdata",
    example = "rvab init -f <config> ",
    example = "rvab init -f <config> --slot a",
    example = "rvab init -full <config> "
)]
/// set necessary gpt layout
struct InitMode {
    /// config file for userdata init
    #[argh(option, short = 'f')]
    config: Option<String>,
    /// config file for full init
    #[argh(option)]
    full: Option<String>,
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
#[argh(
    subcommand,
    name = "install",
    description = "install(update) metadata for all slots (all slots' metadata must keep synced)",
    example = "rvab install -u <config>",
    example = "rvab install -t <output template file> --exclude <exclude list file> --dynpt <dyn partitions list file>",
    example = "rvab install -c <config>"
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
#[argh(subcommand, name = "list", example = "rvab list [slot]")]
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

#[derive(FromArgs)]
#[argh(subcommand, name = "test")]
/// test mode
struct TestMode {}
fn main() {
    let args: CmdProg = argh::from_env();
    match args.mode {
        Mode::Init(init) => {
            println!("Init mode");
            if let Some(out) = init.template {
                let path = std::path::PathBuf::from(out);
                generate_template_init_config_file(path, init.exclude, init.dynpt).unwrap();
                return;
            }
            if let Some(check) = init.check {
                check_slots_config(&check);
                return;
            }
            //only init userdata
            if let Some(config) = init.config {
                let ret = try_init_userdata_partition(&config, &init.slot, args.silent);
                if ret.is_err() {
                    eprintln!("Init failed ");
                }
                return;
            }
            if let Some(config) = init.full {
                let ret = try_init_partition_table_layout(&config, &init.slot, true, args.silent);
                if ret.is_err() {
                    eprintln!("Init failed {}", ret.err().unwrap());
                }
                println!("Done , please keep your config , reboot to run install mode");
                return;
            }
            println!("Option required");
            return;
        }
        Mode::Install(install) => {
            println!("Install mode");
            if let Some(out) = install.template {
                let path = std::path::PathBuf::from(out);
                generate_template_init_config_file(path, install.exclude, install.dynpt).unwrap();
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
        Mode::Test(_) => {
            println!("Test mode");
            test_indicatif();
        }
    }
}

pub fn test_indicatif() {
    test_indicatif_multi();
}
pub fn test_indicatif_download() {
    let mut downloaded = 0;
    let total_size = 231231231;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));

    while downloaded < total_size {
        let new = min(downloaded + 223211, total_size);
        downloaded = new;
        pb.set_position(new);
        thread::sleep(Duration::from_millis(12));
    }

    pb.finish_with_message("downloaded");
}

pub fn test_indicatif_multi() {
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("##-");

    let n = 200;
    let pb = m.add(ProgressBar::new(n));
    pb.set_style(sty.clone());
    pb.set_message("todo");
    let pb2 = m.add(ProgressBar::new(n));
    pb2.set_style(sty.clone());
    pb2.set_message("finished");

    m.println("starting!").unwrap();

    let mut threads = vec![];

    for i in 0..n {
        thread::sleep(Duration::from_millis(15));
        if i == n / 3 {
            thread::sleep(Duration::from_secs(2));
        }
        pb.inc(1);
        let m = m.clone();
        let pb2 = pb2.clone();
        threads.push(thread::spawn(move || {
            let spinner = m.add(ProgressBar::new_spinner().with_message(i.to_string()));
            spinner.enable_steady_tick(Duration::from_millis(100));
            thread::sleep(
                rand::thread_rng().gen_range(Duration::from_secs(1)..Duration::from_secs(5)),
            );
            pb2.inc(1);
        }));
    }
    pb.finish_with_message("all jobs started");

    for thread in threads {
        let _ = thread.join();
    }
    pb2.finish_with_message("all jobs done");
    m.clear().unwrap();
}
