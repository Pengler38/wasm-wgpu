use std::path::Path;
use std::env;
use std::fs;

fn main() {
    println!("cargo::rerun-if-changed=src/");
    // Find the linecount of all .rs files to display on our webpage
    let out_dir = env::var("OUT_DIR").unwrap();
    let linecount_path = Path::new(&out_dir).join("linecount.txt");
        
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_path = Path::new(&cargo_manifest_dir).join("src/");
        
    let linecount = parse_dir(&src_path);
    let linecount_string = "\"".to_string() + &linecount.to_string() + "\"";
    fs::write(&linecount_path, linecount_string).unwrap();
}

// Check all items in the directory, recurse on a directory, call count_lines on a .rs file
fn parse_dir(dir_path: &Path) -> u32 {
    let mut linecount = 0;
    for opt_item in fs::read_dir(dir_path).unwrap() {
        let item_path = opt_item.unwrap().path();
        if item_path.is_dir() {
            linecount += parse_dir(item_path.as_path());
        } else if item_path.to_str().unwrap().ends_with(".rs") {
            linecount += count_lines(item_path.as_path());
        }
    }
    linecount
}

fn count_lines(path: &Path) -> u32 {
    let f = fs::read_to_string(path).unwrap();
    let mut lines = 0;
    for c in f.chars() {
        if c == '\n' {
            lines += 1;
        }
    }
    lines
}
