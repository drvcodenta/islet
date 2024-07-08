use crate::record::{UnsafeItem, UnsafeKind};

use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::{self, FnKind, Visitor};
use rustc_hir::BlockCheckMode::UnsafeBlock;
use rustc_hir::{
    Block, BodyId, Expr, ExprKind, FnDecl, ImplItem, Item, ItemKind, QPath, TraitFn, TraitItem,
    UnsafeSource, Unsafety,
};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

use std::collections::{HashMap, HashSet};

pub struct Analyzer<'tcx> {
    tcx: TyCtxt<'tcx>,
    unsafe_items: Vec<UnsafeItem>,
    call_graph: HashMap<String, Vec<String>>,
}

impl<'tcx> Analyzer<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            unsafe_items: Vec::new(),
            call_graph: HashMap::new(),
        }
    }

    pub fn print_unsafe_items(&self) {
        println!("Unsafe items:");
        for item in &self.unsafe_items {
            println!("  - {:?}", item);
        }
    }

    pub fn print_unsafe_items_verbose(&self) {
        let mut items = self.unsafe_items.clone();
        items.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.cmp(&b.name)));

        for item in items {
            println!("{:?} - {}", item.kind, item.name);
        }
    }

    pub fn print_call_graph(&self) {
        println!("Call graph:");
        for (caller, callees) in &self.call_graph {
            println!("{} calls:", caller);
            for callee in callees {
                println!("  - {}", callee);
            }
        }
    }
}

impl<'tcx> Visitor<'tcx> for Analyzer<'tcx> {
    fn visit_block(&mut self, block: &'tcx Block<'tcx>) {
        if block.rules == UnsafeBlock(UnsafeSource::UserProvided) {
            let owner_id = self.tcx.hir().get_parent_item(block.hir_id);
            let def_path = self.tcx.def_path(owner_id.into());
            let crate_name = self.tcx.crate_name(def_path.krate).to_string();
            let mut fn_name = def_path.to_string_no_crate_verbose();
            if fn_name.contains("impl") {
                fn_name = format!("::{}", self.tcx.def_path_str(owner_id));
            };
            self.unsafe_items.push(UnsafeItem::new(
                UnsafeKind::Block,
                format!("{}{}", crate_name, fn_name),
            ));
        }
        intravisit::walk_block(self, block);
    }

    fn visit_impl_item(&mut self, item: &'tcx ImplItem<'tcx>) {
        if let rustc_hir::ImplItemKind::Fn(_, body_id) = &item.kind {
            let body = self.tcx.hir().body(*body_id);
            self.visit_body(body);
        }
        intravisit::walk_impl_item(self, item);
    }

    fn visit_trait_item(&mut self, item: &'tcx TraitItem<'tcx>) {
        if let rustc_hir::TraitItemKind::Fn(fn_sig, trait_fn) = &item.kind {
            if let TraitFn::Provided(body_id) = trait_fn {
                let body = self.tcx.hir().body(*body_id);
                self.visit_body(body);
            }

            if let TraitFn::Required(_) = trait_fn {
                if fn_sig.header.unsafety == Unsafety::Unsafe {
                    let def_path = self.tcx.def_path(item.owner_id.to_def_id());
                    let crate_name = self.tcx.crate_name(def_path.krate).to_string();
                    let fn_name = def_path.to_string_no_crate_verbose();
                    self.unsafe_items.push(UnsafeItem::new(
                        UnsafeKind::Function,
                        format!("{}{}", crate_name, fn_name),
                    ));
                }
            }
        }
        intravisit::walk_trait_item(self, item);
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(path_expr, _) = &expr.kind {
            if let ExprKind::Path(QPath::Resolved(_, path)) = &path_expr.kind {
                if let Some(def_id) = path.res.opt_def_id() {
                    let owner_id = self.tcx.hir().get_parent_item(expr.hir_id);
                    let caller = self
                        .tcx
                        .def_path(owner_id.into())
                        .to_string_no_crate_verbose();
                    let callee = self.tcx.def_path(def_id).to_string_no_crate_verbose();
                    self.call_graph.entry(caller).or_default().push(callee);
                }
            }
        }
        intravisit::walk_expr(self, expr);
    }

    fn visit_fn(
        &mut self,
        fk: FnKind<'tcx>,
        fd: &'tcx FnDecl<'tcx>,
        b: BodyId,
        _: Span,
        id: LocalDefId,
    ) {
        let header = match fk {
            intravisit::FnKind::ItemFn(_, _, header) => header,
            intravisit::FnKind::Method(_, sig) => sig.header,
            _ => return,
        };

        if header.unsafety == Unsafety::Unsafe {
            let def_path = self.tcx.def_path(id.to_def_id());
            let crate_name = self.tcx.crate_name(def_path.krate).to_string();
            let mut fn_name = def_path.to_string_no_crate_verbose();
            if fn_name.contains("impl") {
                fn_name = format!("::{}", self.tcx.def_path_str(id));
            };
            self.unsafe_items.push(UnsafeItem::new(
                UnsafeKind::Function,
                format!("{}{}", crate_name, fn_name),
            ));
        }

        intravisit::walk_fn(self, fk, fd, b, id);
    }

    fn visit_item(&mut self, item: &'tcx Item<'tcx>) {
        if let ItemKind::Fn(_, _, body_id) = &item.kind {
            let body = self.tcx.hir().body(*body_id);
            self.visit_body(body);
        }

        if let ItemKind::Trait(_, unsafety, _, _, _) = &item.kind {
            if *unsafety == Unsafety::Unsafe {
                let def_path = self.tcx.def_path(item.owner_id.to_def_id());
                let crate_name = self.tcx.crate_name(def_path.krate).to_string();
                let trait_name = def_path.to_string_no_crate_verbose();
                self.unsafe_items.push(UnsafeItem::new(
                    UnsafeKind::Trait,
                    format!("{}{}", crate_name, trait_name),
                ));
            }
        }

        if let ItemKind::Impl(ref_) = &item.kind {
            if ref_.unsafety == Unsafety::Unsafe {
                let def_path = self.tcx.def_path(item.owner_id.to_def_id());
                let crate_name = self.tcx.crate_name(def_path.krate).to_string();
                let impl_name = format!("{}", self.tcx.def_path_str(item.owner_id));
                self.unsafe_items.push(UnsafeItem::new(
                    UnsafeKind::Impl,
                    format!("{}::{}", crate_name, impl_name),
                ));
            }
        }

        intravisit::walk_item(self, item);
    }
}
