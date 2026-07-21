use std::fs::File;
use std::io::Write;

fn main() {
    let mut file = File::create("K:/Evelyn-lyn/.artifacts/f1a93a1c-38e8-42f6-8f24-774b65f92323/scratch/version_err.lync").unwrap();
    file.write_all(b"SYL\0").unwrap();
    file.write_all(&[2, 0, 0, 0]).unwrap();
}
