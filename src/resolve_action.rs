use crate::{load_workspace, Repository};
use std::{collections::HashMap, iter::zip};

fn reduce(path_a: String, path_b: String) -> (String, String) {
    let mut ret_a = Vec::new();
    let mut ret_b = Vec::new();
    for (a, b) in zip(
        path_a.split('/').collect::<Vec<&str>>(),
        path_b.split('/').collect::<Vec<&str>>(),
    )
    .rev()
    {
        ret_a.insert(0, a);
        ret_b.insert(0, b);
        if a != b {
            break;
        }
    }

    (ret_a.join("/"), ret_b.join("/"))
}

fn reduce_repo_names(
    repositories: Vec<Repository>,
) -> HashMap<String, Repository> {
    let mut ret: HashMap<String, Repository> = HashMap::new();

    for repository in repositories {
        let name = repository.name.clone();
        let name = String::from(name.split('/').next_back().unwrap());

        if let Some(conflict) = ret.remove(&name) {
            let (conflict_name, name) =
                reduce(conflict.name.clone(), repository.name.clone());
            ret.insert(conflict_name, conflict);
            ret.insert(name, repository);
        } else {
            ret.insert(name, repository);
        }
    }

    ret
}

pub fn resolve(repo_id: String) -> i32 {
    let _ = repo_id;
    let repositories = reduce_repo_names(load_workspace());

    let mut names = repositories.keys().collect::<Vec<&String>>();
    names.sort();

    for name in names {
        println!(
            "{}: {}",
            name,
            repositories.get(name).unwrap().root.display()
        );
    }

    0
}
