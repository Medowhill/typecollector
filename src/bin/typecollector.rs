use std::{collections::BTreeMap, fs, path::PathBuf};

use clap::Parser;
use typecollector::compiler;

#[derive(Parser, Debug)]
struct Args {
    input: PathBuf,
}

fn main() {
    let args = Args::parse();

    let mut functions = vec![];
    for path in files(args.input, "rs") {
        if path.ends_with("tinycc/bitfields.rs") {
            continue;
        }
        if let Ok(code) = fs::read_to_string(path) {
            let code = if code.contains("extern crate libc;") {
                code
            } else {
                format!("extern crate libc;{}", code)
            };
            functions.extend(compiler::run(&code));
        }
    }

    let n = functions.len();
    let n1 = functions.iter().filter(|(_, tys)| tys.is_empty()).count();
    let n2 = functions
        .iter()
        .filter(|(_, tys)| !tys.is_empty() && tys.iter().all(|ty| compiler::is_c_type(ty)))
        .count();
    let n3 = functions
        .iter()
        .filter(|(_, tys)| !tys.is_empty() && tys.iter().all(|ty| !compiler::is_c_type(ty)))
        .count();
    let n4 = functions
        .iter()
        .filter(|(_, tys)| {
            !tys.is_empty()
                && tys.iter().any(|ty| compiler::is_c_type(ty))
                && tys.iter().any(|ty| !compiler::is_c_type(ty))
        })
        .count();

    let mut rtys: BTreeMap<&str, usize> = BTreeMap::new();
    for ty in functions.iter().flat_map(|(_, tys)| tys) {
        if !compiler::is_c_type(ty) {
            *rtys.entry(ty).or_default() += 1;
        }
    }
    let rty_n = rtys.values().sum::<usize>();
    let rty_kind_n = rtys.len();

    let mut ctys: BTreeMap<&str, usize> = BTreeMap::new();
    for ty in functions.iter().flat_map(|(_, tys)| tys) {
        if compiler::is_c_type(ty) {
            *ctys.entry(ty).or_default() += 1;
        }
    }
    let cty_n = ctys.values().sum::<usize>();
    let cty_kind_n = ctys.len();

    let rtys_str = rtys
        .into_iter()
        .map(|(ty, n)| format!("{} {}", ty, n))
        .collect::<Vec<_>>()
        .join(", ");
    let ctys_str = ctys
        .into_iter()
        .map(|(ty, n)| format!("{} {}", ty, n))
        .collect::<Vec<_>>()
        .join(", ");

    println!(
        "{} {} {} {} {} {} {} {} {}",
        n, n1, n2, n3, n4, rty_n, rty_kind_n, cty_n, cty_kind_n
    );
    println!("{}", rtys_str);
    println!("{}", ctys_str);
}

fn files(path: PathBuf, ext: &str) -> Vec<PathBuf> {
    if path.is_dir() {
        fs::read_dir(path)
            .unwrap()
            .flat_map(|entry| files(entry.unwrap().path(), ext))
            .collect()
    } else if path.extension().and_then(|x| x.to_str()) == Some(ext) {
        vec![path]
    } else {
        vec![]
    }
}
