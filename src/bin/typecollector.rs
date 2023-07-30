use std::{collections::BTreeMap, fs, path::PathBuf};

use clap::Parser;
use typecollector::compiler;

#[derive(Parser, Debug)]
struct Args {
    input: PathBuf,
}

fn main() {
    let args = Args::parse();

    let code = fs::read_to_string(args.input).unwrap();
    let functions = compiler::run(&code);

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

    let mut tys: BTreeMap<String, usize> = BTreeMap::new();
    for ty in functions.into_iter().flat_map(|(_, tys)| tys) {
        if !compiler::is_c_type(&ty) {
            *tys.entry(ty).or_default() += 1;
        }
    }
    let ty_n = tys.iter().map(|(_, n)| n).sum::<usize>();
    let ty_kind_n = tys.len();

    println!("{} {} {} {} {} {} {}", n, n1, n2, n3, n4, ty_n, ty_kind_n);

    let tys_str = tys
        .into_iter()
        .map(|(ty, n)| format!("{} {}", ty, n))
        .collect::<Vec<_>>()
        .join(", ");
    println!("{}", tys_str);
}
