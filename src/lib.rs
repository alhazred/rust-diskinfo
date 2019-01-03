use std::fs::{ File, read_dir, read_link };
use std::mem::zeroed;
use std::os::unix::io::AsRawFd;
use std::io::{ BufReader, BufRead, Error };

extern crate libc;
use libc::{ c_int, ioctl };

extern crate kstat;
use kstat::{KstatData, KstatReader};
use kstat::kstat_named::KstatNamedData;


const DKIOCREMOVABLE: i32 = 1040;
const DKIOCGMEDIAINFO: i32 = 1066;
const DKIOCINFO: i32 = 1027;

pub type ULonglongT = ::std::os::raw::c_ulonglong;
pub type DiskaddrT = ULonglongT;
pub type UChar = ::std::os::raw::c_uchar;
pub type UShortT = ::std::os::raw::c_ushort;
pub type UIntT = ::std::os::raw::c_uint;


#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DkMinfo {
    pub dki_media_type: UIntT,
    pub dki_lbsize: UIntT,
    pub dki_capacity: DiskaddrT,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DkCinfo {
    pub dki_cname: [::std::os::raw::c_char; 16usize],
    pub dki_ctype: UShortT,
    pub dki_flags: UShortT,
    pub dki_cnum: UShortT,
    pub dki_addr: UIntT,
    pub dki_space: UIntT,
    pub dki_prio: UIntT,
    pub dki_vec: UIntT,
    pub dki_dname: [::std::os::raw::c_char; 16usize],
    pub dki_unit: UIntT,
    pub dki_slave: UIntT,
    pub dki_partition: UShortT,
    pub dki_maxtransfer: UShortT,
}

pub fn get_removable(fd: c_int) -> i64 {
    let mut removable = 0;
    let res = unsafe { ioctl(fd, DKIOCREMOVABLE, &mut removable) };
    if res == -1 {
        panic!("DKIOCREMOVABLE err")
    }
    removable
}

pub fn get_ctype(fd: c_int) -> String {
    let mut dkinfo:DkCinfo = unsafe { zeroed() };
    let mut ctype:String = String::new(); 
    let res = unsafe { ioctl(fd, DKIOCINFO, &mut dkinfo) };
    if res == -1 {
        panic!("DKIOCINFO err");
    }
    match dkinfo.dki_ctype {
      20 => ctype.push_str("ATA"),
      13 => ctype.push_str("SCSI"),
      _ => ctype.push_str("UNKNOWN"),
    }
    ctype
}

pub fn get_media(fd: c_int) -> f64 {
    let mut media: DkMinfo = unsafe { zeroed() };
    let res = unsafe { ioctl(fd, DKIOCGMEDIAINFO, &mut media) };
    if res == -1 {
        panic!("DKIOMEDIA err")
    }
    let size = ((media.dki_lbsize as u64 * media.dki_capacity) as f64) / 1024.0 / 1024.0 / 1024.0;
    size
}

fn format_ks(m: &KstatNamedData) -> String {
    let s = format!("{:?}!", m);
    let tokens: Vec<_> = s.split('"').collect();
    tokens[1].to_string()
}

fn get_kstat(sd: &str) -> Vec<KstatData> {
    let mut kstat_val:String = String::new();
    let dkerr = format!("{},err", sd);
    let reader = KstatReader::new(None, None, Some(dkerr), None)
        .expect("failed to create kstat reader");
    reader.read().expect("failed to read kstats")
}

fn get_kstat_value(k: &Vec<KstatData>, field: &str) -> String {
    let mut kstat_val:String = String::new();
    for stat in k {
       kstat_val = format_ks(&stat.data[field]);
      }
    kstat_val
}

fn print_disk(path: &str, fname: File, disk: &str) { 
    let ctype = get_ctype(fname.as_raw_fd());
    let res = get_removable(fname.as_raw_fd());
    if res == 0 {
        let dksize = get_media(fname.as_raw_fd());
        let sympath = read_link(&path).unwrap();
        let devpath = sympath.into_os_string().into_string().unwrap(); 
        let tokens: Vec<&str> = devpath.split("/").collect();
        let sdpart = tokens[tokens.len()-1];
        let sdnum: Vec<&str> = sdpart.split(":").collect();
        let pathinst = File::open("/etc/path_to_inst").unwrap();
        let file = BufReader::new(&pathinst);
        for line in file.lines() {
            let l = line.unwrap();
            if l.contains(sdnum[0]) && l.contains(tokens[tokens.len()-2]) {
	        let sdtok: Vec<&str> = l.split(" ").collect();
	        let sd = format!("sd{}", sdtok[sdtok.len()-2]); 
	        let kstats = get_kstat(&sd);
		let serial = get_kstat_value(&kstats, "Serial No");
	        let product = get_kstat_value(&kstats, "Product");
	        let vendor = get_kstat_value(&kstats, "Vendor");
	        println!("{: <7} {: <23} {: <8} {: <16} {: <20} {:.2?} GiB",
		    ctype, disk, vendor, product, serial, dksize);
            }
	}
    }
}

pub fn get_disks() -> Result<(), Box<Error>> {
    let dir = "/dev/rdsk";
    println!("TYPE    DISK                    VID      PID              SERIAL               SIZE");
    for entry in read_dir(dir).unwrap() { 
        let post_path = entry.unwrap().path();
        let fname = post_path.file_name().unwrap().to_str().unwrap();
        if fname.ends_with("p0") { 
           let mut disk = String::from(fname);
           let len = disk.len();
           disk.truncate(len - 2); 
           let path = ["/dev/rdsk", fname].join("/"); 
           let _rawfd = match File::open(&path) {
               Ok(rawfd) => print_disk(&path, rawfd, &disk),
               Err(_error) => (),
           };
        }
    }
    Ok(())
}

