//! Make Git ask rt for the credentials of the hosts with tokens.
use clap::Args;

use crate::config::Config;
use crate::ui::Ui;
use crate::ui_error;
use crate::version_control_system::git::config::credential_helper_config;
use crate::version_control_system::git::config::setup_credential_helper;

/// Configure Git, in the global configuration, to ask rt for the credentials
/// of the hosts the identities have Git tokens for.
#[derive(Args)]
pub struct SetupGitArgs {
    /// Only print the configuration, without writing it.
    #[arg(short = 'n', long)]
    dry_run: bool,
}

/// Execute the `rt identity setup-git` command.
pub fn run(ui: &Ui<'_>, config: &Config, args: &SetupGitArgs) -> i32 {
    let hosts = config.identities.git_token_hosts();

    if hosts.is_empty() {
        ui.error("No identity has a Git token");
        return 1;
    }

    for host in hosts {
        print!("{}", credential_helper_config(host));

        if !args.dry_run
            && let Err(err) = setup_credential_helper(host)
        {
            ui_error!(ui, "Unable to configure Git for {host}: {err}");
            return 1;
        }
    }

    0
}
