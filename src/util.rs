use std::path::{Component, Path, PathBuf};

pub fn path_canon(path: &Path) -> PathBuf {
    let mut v = Vec::new();
    for i in path.components() {
        match i {
            Component::Prefix(_) => {
                panic!("should not have prefix")
            }
            Component::RootDir => {
                panic!("should not have root dir")
            }
            Component::CurDir => {}
            Component::ParentDir => {
                v.pop();
            }
            Component::Normal(n) => v.push(n),
        }
    }

    let mut out = PathBuf::new();
    for p in v {
        out.push(p)
    }
    out
}
