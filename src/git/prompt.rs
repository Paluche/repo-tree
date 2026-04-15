//! Build the shell prompt information for Git repositories.
use std::path::Path;

use colored::Colorize;

use crate::config::Config;
use crate::git;
use crate::prompt::Prompt;
use crate::prompt::PromptListField;

/// Build the shell prompt information for Git repositories.
pub fn prompt(
    config: &Config,
    prompt: &mut Prompt,
    root: &Path,
    is_jj_colocated: bool,
) -> i32 {
    let git_status = git::status(&root.to_path_buf()).unwrap();
    let config = &config.prompt.git;

    prompt.push(
        config
            .ongoing_operations
            .display(&git_status.ongoing_operations),
    );

    let (staged, unstaged, submodules) = git_status.short_status();

    if !is_jj_colocated {
        {
            let mut field = PromptListField::new(" ");
            field.push(config.branches.display(&git_status.head.branches));
            field.push(config.tags.display(&git_status.head.tags));
            field.push(
                if let Some(upstream_info) = &git_status.head.upstream {
                    if upstream_info.gone {
                        config.upstream.gone()
                    } else if upstream_info.ahead == 0
                        && upstream_info.behind == 0
                    {
                        config.upstream.up_to_date()
                    } else if upstream_info.ahead != 0
                        && upstream_info.behind != 0
                    {
                        config.upstream.diverged()
                    } else if upstream_info.ahead != 0 {
                        config.upstream.ahead()
                    } else {
                        config.upstream.behind()
                    }
                } else if git_status.head.branch == "(detached)" {
                    config.upstream.detached()
                } else {
                    config.upstream.local()
                },
            );
            prompt.push(field);
        }

        prompt.push(format!(
            "{}{}",
            staged.as_string().green(),
            unstaged.as_string().red()
        ));
    }

    // Submodule status.
    prompt.push(submodules.as_string().red());

    // stash status
    if git_status.nb_stash != 0 {
        prompt.push(&config.stash);
    }

    0
}
