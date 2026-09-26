mod jujutsu_command_arg;

pub use forbidden_repo_arg::FORBIDDEN_REPO_ARG;

store.register_early_pass(|| Box::new(ForbiddenRepoArg));
