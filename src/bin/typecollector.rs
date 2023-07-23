use std::{collections::BTreeMap, fs, path::PathBuf};

use clap::Parser;
use typecollector::compiler::run;

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
            functions.extend(run(&code));
        }
    }

    functions.retain(|(_, tys)| !tys.is_empty());
    println!("{}", functions.len());

    let mut tys: BTreeMap<String, usize> = BTreeMap::new();
    for ty in functions.into_iter().flat_map(|(_, tys)| tys) {
        *tys.entry(ty).or_default() += 1;
    }
    println!("{}", tys.len());

    let tys_str = tys
        .into_iter()
        .map(|(ty, n)| format!("{} {}", ty, n))
        .collect::<Vec<_>>()
        .join(", ");
    println!("{}", tys_str);
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
