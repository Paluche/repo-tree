use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_literal;
use rustc_ast::ast::LitKind;
use rustc_hir::{Expr, ExprKind, HirId};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Warns when `arg("--repository")` is used instead of `repository(...)`.
    ///
    /// ### Why is this bad?
    /// This is easy to misuse and bypasses the typed API.
    ///
    /// ### Example
    /// ```rust,ignore
    /// cmd.arg("--repository");
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// cmd.repository("/path/to/repo");
    /// ```
    pub FORBIDDEN_REPO_ARG,
    restriction,
    "detects `arg(\"--repository\")` and suggests `repository(...)`"
}

declare_lint_pass!(ForbiddenRepoArg => [FORBIDDEN_REPO_ARG]);

impl<'tcx> LateLintPass<'tcx> for ForbiddenRepoArg {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, args, _) = expr.kind else {
            return;
        };

        // Match method name "arg"
        if path.ident.name.as_str() != "arg" {
            return;
        }

        // We only care about the first argument, like: cmd.arg("--repository")
        let Some(arg_expr) = args.first() else {
            return;
        };

        // Extract literal string value
        let Some(arg_str) = literal_str_value(cx, arg_expr) else {
            return;
        };

        if arg_str == "--repository" {
            span_lint_and_help(
                cx,
                FORBIDDEN_REPO_ARG,
                expr.span,
                "use `repository(...)` instead of `arg(\"--repository\")`",
                None,
                "The `repository()` helper is the typed API for this option.",
            );
        }
    }
}

fn literal_str_value<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<String> {
    let ExprKind::Lit(lit) = expr.kind else {
        return None;
    };

    match lit.node {
        rustc_ast::ast::LitKind::Str(s, _) => Some(s.as_str().to_string()),
        _ => None,
    }
}
