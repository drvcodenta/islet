#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

mod analyzer;
mod record;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::{interface::Compiler, Queries};

use crate::analyzer::Analyzer;

struct Plugin;

impl Callbacks for Plugin {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        queries.global_ctxt().unwrap().enter(|tcx| {
            let mut analyzer = Analyzer::new(tcx);
            tcx.hir().visit_all_item_likes_in_crate(&mut analyzer);

            //analyzer.print_unsafe_items();
            analyzer.print_unsafe_items_verbose();
            //            analyzer.print_call_graph();
        });

        Compilation::Continue
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    rustc_driver::RunCompiler::new(&args, &mut Plugin)
        .run()
        .unwrap();
}
