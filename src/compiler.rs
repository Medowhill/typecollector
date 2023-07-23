use std::{collections::BTreeSet, path::PathBuf, process::Command};

use rustc_data_structures::sync::Lrc;
use rustc_errors::{
    emitter::Emitter, registry::Registry, translation::Translate, FluentBundle, Handler,
};
use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::{
    def::Res,
    def_id::DefId,
    intravisit::{self, Visitor},
    ItemKind, PrimTy, QPath, TraitRef, Ty, TyKind,
};
use rustc_interface::{interface::Compiler, Config};
use rustc_middle::{hir::nested_filter, ty::TyCtxt};
use rustc_session::{
    config::{CheckCfg, Input, Options},
    parse::ParseSess,
};
use rustc_span::source_map::{FileName, SourceMap};

struct NoEmitter;

impl Translate for NoEmitter {
    fn fluent_bundle(&self) -> Option<&Lrc<FluentBundle>> {
        None
    }

    fn fallback_fluent_bundle(&self) -> &FluentBundle {
        panic!()
    }
}

impl Emitter for NoEmitter {
    fn emit_diagnostic(&mut self, _: &rustc_errors::Diagnostic) {}

    fn source_map(&self) -> Option<&Lrc<SourceMap>> {
        None
    }
}

struct TypeVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    types: BTreeSet<String>,
}

impl<'tcx> TypeVisitor<'tcx> {
    fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            types: BTreeSet::new(),
        }
    }

    fn add<S: AsRef<str>>(&mut self, s: S) {
        self.types.insert(s.as_ref().to_string());
    }

    fn def_id_to_string(&self, def_id: DefId) -> Option<String> {
        if !def_id.is_local() {
            let cstore = self.tcx.cstore_untracked();
            let krate = cstore.crate_name(def_id.krate);
            let krate = krate.as_str();
            let path = self.tcx.def_path(def_id);
            if krate == "std" || krate == "alloc" || krate == "core" {
                let ty = format!("{}{}", krate, path.to_string_no_crate_verbose());
                if !ty.starts_with("std::os::raw::") && !ty.starts_with("core::ffi::") {
                    return Some(ty);
                }
            }
        }
        None
    }
}

impl<'tcx> Visitor<'tcx> for TypeVisitor<'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.tcx.hir()
    }

    fn visit_trait_ref(&mut self, t: &'tcx TraitRef<'tcx>) {
        if let Some(def_id) = t.trait_def_id() {
            if let Some(name) = self.def_id_to_string(def_id) {
                self.add(name);
            }
        }
        intravisit::walk_trait_ref(self, t)
    }

    fn visit_ty(&mut self, ty: &'tcx Ty<'tcx>) {
        match &ty.kind {
            TyKind::Slice(_) => self.add("primitive::slice"),
            TyKind::Ref(_, _) => self.add("primitive::ref"),
            TyKind::Never => self.add("primitive::never"),
            TyKind::Tup(tys) if tys.len() >= 2 => self.add("primitive::tuple"),
            TyKind::Path(QPath::Resolved(_, path)) => match path.res {
                Res::Def(_, def_id) => {
                    if let Some(name) = self.def_id_to_string(def_id) {
                        self.add(name);
                    }
                }
                Res::PrimTy(PrimTy::Str) => self.add("primitive::str"),
                _ => (),
            },
            _ => (),
        }
        intravisit::walk_ty(self, ty);
    }
}

pub fn run(code: &str) -> Vec<(String, BTreeSet<String>)> {
    let functions = run_compiler(make_config(code), |compiler| {
        compiler.enter(|queries| {
            queries.global_ctxt().ok()?.enter(|tcx| {
                let hir = tcx.hir();
                let mut functions = vec![];
                for id in hir.items() {
                    let item = hir.item(id);
                    if let ItemKind::Fn(sig, gen, _) = &item.kind {
                        let def_path = tcx.def_path(item.owner_id.to_def_id());
                        let def_path = def_path.to_string_no_crate_verbose();
                        if def_path.starts_with("::__laertes_array::")
                            || def_path.starts_with("::laertes_rt::")
                        {
                            continue;
                        }
                        let name = item.ident.name.to_ident_string();
                        let mut visitor = TypeVisitor::new(tcx);
                        visitor.visit_fn_decl(sig.decl);
                        visitor.visit_generics(gen);
                        functions.push((name, visitor.types))
                    }
                }
                Some(functions)
            })
        })
    })
    .flatten();
    functions.unwrap_or_default()
}

fn run_compiler<R: Send, F: FnOnce(&Compiler) -> R + Send>(config: Config, f: F) -> Option<R> {
    rustc_driver::catch_fatal_errors(|| rustc_interface::run_compiler(config, f)).ok()
}

fn make_config(code: &str) -> Config {
    Config {
        opts: Options {
            maybe_sysroot: Some(PathBuf::from(sys_root())),
            ..Options::default()
        },
        crate_cfg: FxHashSet::default(),
        crate_check_cfg: CheckCfg::default(),
        input: Input::Str {
            name: FileName::Custom("main.rs".to_string()),
            input: code.to_string(),
        },
        output_dir: None,
        output_file: None,
        file_loader: None,
        locale_resources: rustc_driver_impl::DEFAULT_LOCALE_RESOURCES,
        lint_caps: FxHashMap::default(),
        parse_sess_created: Some(Box::new(|ps: &mut ParseSess| {
            ps.span_diagnostic = Handler::with_emitter(true, None, Box::new(NoEmitter));
        })),
        register_lints: None,
        override_queries: None,
        make_codegen_backend: None,
        registry: Registry::new(rustc_error_codes::DIAGNOSTICS),
    }
}

fn sys_root() -> String {
    std::env::var("SYSROOT")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let home = std::env::var("RUSTUP_HOME")
                .or_else(|_| std::env::var("MULTIRUST_HOME"))
                .ok();
            let toolchain = std::env::var("RUSTUP_TOOLCHAIN")
                .or_else(|_| std::env::var("MULTIRUST_TOOLCHAIN"))
                .ok();
            toolchain_path(home, toolchain)
        })
        .or_else(|| {
            Command::new("rustc")
                .arg("--print")
                .arg("sysroot")
                .output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
        })
        .or_else(|| option_env!("SYSROOT").map(PathBuf::from))
        .or_else(|| {
            let home = option_env!("RUSTUP_HOME")
                .or(option_env!("MULTIRUST_HOME"))
                .map(ToString::to_string);
            let toolchain = option_env!("RUSTUP_TOOLCHAIN")
                .or(option_env!("MULTIRUST_TOOLCHAIN"))
                .map(ToString::to_string);
            toolchain_path(home, toolchain)
        })
        .map(|pb| pb.to_string_lossy().to_string())
        .unwrap()
}

fn toolchain_path(home: Option<String>, toolchain: Option<String>) -> Option<PathBuf> {
    home.and_then(|home| {
        toolchain.map(|toolchain| {
            let mut path = PathBuf::from(home);
            path.push("toolchains");
            path.push(toolchain);
            path
        })
    })
}
