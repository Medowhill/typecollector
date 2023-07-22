use std::{collections::BTreeSet, fs, path::PathBuf};

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
        println!("{:?}", path);
        if let Ok(code) = fs::read_to_string(path) {
            functions.extend(run(&code));
        }
    }
    for (name, tys) in &functions {
        println!("{}: {:?}", name, tys);
    }
    let tys: BTreeSet<_> = functions.into_iter().flat_map(|(_, tys)| tys).collect();
    for ty in &tys {
        println!("{}", ty);
    }
    println!("{}", tys.len());
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
