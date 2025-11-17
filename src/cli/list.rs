use crate::{Config, UrlParser, get_repo_tree_dir, load_repo_tree};

pub fn list() -> i32 {
    let repositories = load_repo_tree(
        &get_repo_tree_dir(),
        &UrlParser::new(&Config::default()),
    )
    .0;

    for repository in repositories {
        println!("{}", repository.root.display());
    }
    0
}
