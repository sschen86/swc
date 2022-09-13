#[cfg(feature = "debug")]
use std::thread;
use std::{
    borrow::Cow,
    fmt,
    fmt::{Debug, Display, Formatter, Write},
    time::Instant,
};

#[cfg(feature = "pretty_assertions")]
use pretty_assertions::assert_eq;
use swc_common::{
    chain,
    pass::{CompilerPass, Optional, Repeated},
};
use swc_ecma_ast::*;
use swc_ecma_transforms_optimization::simplify::{
    dead_branch_remover, expr_simplifier, ExprSimplifierConfig,
};
use swc_ecma_visit::{as_folder, noop_visit_mut_type, VisitMut, VisitMutWith, VisitWith};
use swc_timer::timer;
use tracing::{debug, error};

pub(crate) use self::pure::{pure_optimizer, PureOptimizerConfig};
use self::{hoist_decls::DeclHoisterConfig, optimize::optimizer};
use crate::{
    analyzer::{analyze, ModuleInfo, UsageAnalyzer},
    compress::hoist_decls::decl_hoister,
    debug::{dump, AssertValid},
    marks::Marks,
    mode::Mode,
    option::CompressOptions,
    util::{now, unit::CompileUnit},
};

mod hoist_decls;
mod optimize;
mod pure;
mod util;

pub(crate) fn compressor<'a, M>(
    module_info: &'a ModuleInfo,
    marks: Marks,
    options: &'a CompressOptions,
    mode: &'a M,
) -> impl 'a + VisitMut
where
    M: Mode,
{
    let compressor = Compressor {
        marks,
        options,
        module_info,
        changed: false,
        pass: 1,
        dump_for_infinite_loop: Default::default(),
        mode,
    };

    chain!(
        as_folder(compressor),
        Optional {
            enabled: options.evaluate || options.side_effects,
            visitor: as_folder(expr_simplifier(
                marks.unresolved_mark,
                ExprSimplifierConfig {}
            ))
        }
    )
}

struct Compressor<'a, M>
where
    M: Mode,
{
    marks: Marks,
    options: &'a CompressOptions,
    module_info: &'a ModuleInfo,
    changed: bool,
    pass: usize,

    dump_for_infinite_loop: Vec<String>,

    mode: &'a M,
}

impl<M> CompilerPass for Compressor<'_, M>
where
    M: Mode,
{
    fn name() -> Cow<'static, str> {
        "compressor".into()
    }
}

impl<M> Compressor<'_, M>
where
    M: Mode,
{
    fn optimize_unit_repeatedly<N>(&mut self, n: &mut N)
    where
        N: CompileUnit
            + VisitWith<UsageAnalyzer>
            + for<'aa> VisitMutWith<Compressor<'aa, M>>
            + VisitWith<AssertValid>,
    {
        trace_op!(
            "Optimizing a compile unit within `{:?}`",
            thread::current().name()
        );

        if self.options.hoist_vars || self.options.hoist_fns {
            let data = analyze(&*n, self.module_info, Some(self.marks));

            let mut v = decl_hoister(
                DeclHoisterConfig {
                    hoist_fns: self.options.hoist_fns,
                    hoist_vars: self.options.hoist_vars,
                    _top_level: self.options.top_level(),
                },
                &data,
            );
            n.apply(&mut v);
            self.changed |= v.changed();
        }

        loop {
            self.changed = false;
            self.optimize_unit(n);
            self.pass += 1;
            if !self.changed {
                break;
            }
        }

        self.pass = 1;
        // let last_mark = n.remove_mark();
        // assert!(
        //     N::is_module() || last_mark == self.marks.standalone,
        //     "{:?}; last={:?}",
        //     self.marks,
        //     last_mark
        // );
    }

    /// Optimize a module. `N` can be [Module] or [FnExpr].
    fn optimize_unit<N>(&mut self, n: &mut N)
    where
        N: CompileUnit
            + VisitWith<UsageAnalyzer>
            + for<'aa> VisitMutWith<Compressor<'aa, M>>
            + VisitWith<AssertValid>,
    {
        let _timer = timer!("optimize", pass = self.pass);

        if self.options.passes != 0 && self.options.passes < self.pass {
            let done = dump(&*n, false);
            debug!("===== Done =====\n{}", done);
            return;
        }

        // This exists to prevent hanging.
        if self.pass > 200 {
            if self.dump_for_infinite_loop.is_empty() {
                error!("Seems like there's an infinite loop");
            }

            let code = n.force_dump();

            if self.dump_for_infinite_loop.contains(&code) {
                let mut msg = String::new();

                for (i, code) in self.dump_for_infinite_loop.iter().enumerate() {
                    let _ = write!(msg, "Code {:>4}:\n\n\n\n\n\n\n\n\n\n{}\n", i, code);

                    // std::fs::write(&format!("pass_{}.js", i), code).unwrap();
                }

                panic!(
                    "Infinite loop detected (current pass = {})\n{}",
                    self.pass, msg
                )
            } else {
                self.dump_for_infinite_loop.push(code);
            }
        }

        #[cfg(feature = "debug")]
        let start = {
            let start = n.dump();
            debug!("===== Start =====\n{}", start);
            start
        };

        {
            tracing::info!(
                "compress: Running expression simplifier (pass = {})",
                self.pass
            );

            let start_time = now();

            let mut visitor = expr_simplifier(self.marks.unresolved_mark, ExprSimplifierConfig {});
            n.apply(&mut visitor);

            self.changed |= visitor.changed();
            if visitor.changed() {
                debug!("compressor: Simplified expressions");
                #[cfg(feature = "debug")]
                {
                    debug!("===== Simplified =====\n{}", dump(&*n, false));
                }
            }

            if let Some(start_time) = start_time {
                let end_time = Instant::now();

                tracing::info!(
                    "compress: expr_simplifier took {:?} (pass = {})",
                    end_time - start_time,
                    self.pass
                );
            }

            #[cfg(feature = "debug")]
            if !visitor.changed() {
                let simplified = n.dump();
                if start != simplified {
                    assert_eq!(
                        DebugUsingDisplay(&start),
                        DebugUsingDisplay(&simplified),
                        "Invalid state: expr_simplifier: The code is changed but changed is not \
                         setted to true",
                    )
                }
            }
        }

        {
            let _timer = timer!("apply pure optimizer");

            let mut visitor = pure_optimizer(
                self.options,
                None,
                self.marks,
                PureOptimizerConfig {
                    enable_join_vars: self.pass > 1,
                    force_str_for_tpl: M::force_str_for_tpl(),
                    #[cfg(feature = "debug")]
                    debug_infinite_loop: self.pass >= 20,
                },
            );
            n.apply(&mut visitor);

            self.changed |= visitor.changed();

            #[cfg(feature = "debug")]
            if visitor.changed() {
                let src = n.dump();
                debug!(
                    "===== Before pure =====\n{}\n===== After pure =====\n{}",
                    start, src
                );
            }
        }

        #[cfg(debug_assertions)]
        {
            n.visit_with(&mut AssertValid);
        }

        if self.options.unused {
            let _timer = timer!("remove dead code");

            let mut visitor = swc_ecma_transforms_optimization::simplify::dce::dce(
                swc_ecma_transforms_optimization::simplify::dce::Config {
                    module_mark: None,
                    top_level: self.options.top_level(),
                    top_retain: self.options.top_retain.clone(),
                },
                self.marks.unresolved_mark,
            );

            loop {
                #[cfg(feature = "debug")]
                let start = n.dump();

                n.apply(&mut visitor);

                self.changed |= visitor.changed();

                #[cfg(feature = "debug")]
                if visitor.changed() {
                    let src = n.dump();
                    debug!(
                        "===== Before DCE =====\n{}\n===== After DCE =====\n{}",
                        start, src
                    );
                }

                if visitor.changed() {
                    self.changed = true;
                } else {
                    break;
                }

                visitor.reset();
            }
        }

        #[cfg(debug_assertions)]
        {
            n.visit_with(&mut AssertValid);
        }

        {
            let _timer = timer!("apply full optimizer");

            let mut data = analyze(&*n, self.module_info, Some(self.marks));

            // TODO: reset_opt_flags
            //
            // This is swc version of `node.optimize(this);`.

            let mut visitor = optimizer(
                self.marks,
                self.options,
                self.module_info,
                &mut data,
                self.mode,
                !self.dump_for_infinite_loop.is_empty(),
            );
            n.apply(&mut visitor);

            self.changed |= visitor.changed();

            // let done = dump(&*n);
            // debug!("===== Result =====\n{}", done);
        }

        if self.options.conditionals || self.options.dead_code {
            #[cfg(feature = "debug")]
            let start = dump(&*n, false);

            let start_time = now();

            let mut v = dead_branch_remover(self.marks.unresolved_mark);
            n.apply(&mut v);

            if let Some(start_time) = start_time {
                let end_time = Instant::now();

                tracing::info!(
                    "compress: dead_branch_remover took {:?} (pass = {})",
                    end_time - start_time,
                    self.pass
                );
            }

            #[cfg(feature = "debug")]
            {
                let simplified = dump(&*n, false);

                if start != simplified {
                    debug!(
                        "===== Removed dead branches =====\n{}\n==== ===== ===== ===== ======\n{}",
                        start, simplified
                    );
                }
            }

            self.changed |= v.changed();
        }
    }
}

impl<M> VisitMut for Compressor<'_, M>
where
    M: Mode,
{
    noop_visit_mut_type!();

    fn visit_mut_module(&mut self, n: &mut Module) {
        self.optimize_unit_repeatedly(n);
    }
}

#[derive(PartialEq, Eq)]
struct DebugUsingDisplay<'a>(pub &'a str);

impl<'a> Debug for DebugUsingDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.0, f)
    }
}
