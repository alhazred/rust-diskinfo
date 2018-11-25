extern crate diskinfo;
use diskinfo::get_disks;

fn main() {
    if let Err(e) = get_disks() {
        eprintln!("error: {}", e);
     }
}
