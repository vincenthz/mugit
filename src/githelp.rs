use git2::{BranchType, Commit, Cred, Oid, RemoteCallbacks, Repository};
use std::collections::{BTreeMap, BTreeSet};

pub fn remote_branches(repo: &Repository) -> git2::Branches {
    repo.branches(Some(BranchType::Remote))
        .expect("remote branches working")
}

pub fn remote_branches_get_name<'a>(repo: &'a Repository, name: &str) -> Option<git2::Branch<'a>> {
    remote_branches(repo)
        .filter_map(|x| match x {
            Ok((b, _)) => match b.name() {
                Ok(Some(s)) if s == name => Some(b),
                _ => None,
            },
            _ => None,
        })
        .next()
}

pub fn remote_resolve_branch<'a>(
    repo: &'a Repository,
    remote_name: &str,
    branch_name: &str,
) -> Result<Commit<'a>, &'static str> {
    let to_find = format!("{}/{}", remote_name, branch_name);
    let branch = remote_branches_get_name(&repo, &to_find).ok_or("remote branch not found")?;
    let branch_ref = branch.into_reference();
    let branch_commit = branch_ref
        .peel_to_commit()
        .map_err(|_| "peel-to-commit fail")?;
    Ok(branch_commit)
}

pub fn all_tags<'a>(repo: &'a Repository) -> BTreeMap<String, Oid> {
    let mut out = BTreeMap::new();
    repo.tag_foreach(|oid, name| {
        match std::str::from_utf8(name) {
            Ok(s) => {
                if let Some(tagname) = s.strip_prefix("refs/tags/") {
                    out.insert(tagname.to_string(), oid);
                } else {
                    ()
                }
            }
            Err(_) => {
                println!(
                    "error all_tags, non unicode tags not supported {} {:x?}",
                    oid, name
                );
            }
        }
        true
    })
    .expect("tag foreach works");
    out
}

pub fn has_remote_branch(repo: &Repository, remote_name: &str, branch: &str) -> bool {
    let remote_branches = remote_branches(repo);
    let known_remote_branches = remote_branches
        .filter_map(|v| v.ok().map(|x| x.0))
        .filter_map(|b| b.name().ok().flatten().map(|x| x.to_string()))
        .collect::<BTreeSet<_>>();

    let to_find = format!("{}/{}", remote_name, branch);

    known_remote_branches.get(&to_find).is_some()
}

pub fn has_remote_tag(repo: &Repository, tag: &str) -> bool {
    let tags = repo.tag_names(None).expect("tags working");
    let known_tags = tags
        .iter()
        .filter_map(|v| v)
        .map(|x| x.to_string())
        .collect::<BTreeSet<_>>();

    known_tags.get(tag).is_some()
}

pub fn remote_callbacks() -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();

    // hardcoded to id_ed25519 file
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            std::path::Path::new(&format!(
                "{}/.ssh/id_ed25519",
                std::env::var("HOME").unwrap()
            )),
            None,
        )
    });
    callbacks
}
