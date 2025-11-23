use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::os::unix::ffi::OsStrExt;
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use veritas_dump::cache::HashBundle;
use crate::n8sim::sorted::SortedNodes;

// The N8 is only capable of 12 pages (of 21 each), plus an 13th page of 2.
// For a total of 254 entries per directory.
pub const MAX_PAGES: usize = 12;
pub const MAX_PER_PAGE: usize = 21;
pub const REC_MAX_DIR_LEN: usize = MAX_PAGES * MAX_PER_PAGE;
pub const ABS_MAX_DIR_LEN: usize = (u16::MAX as usize + 1) / 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteFile {
    path: Utf8PathBuf,
    hashes: HashSet<HashBundle>,
}

/// Maps the layout of a filesystem, without containing the files' contents.
/// 
/// After registering files from local storage or from an existing SD card, the mapper can
/// create a new filesystem compatible with the N8 flashcart. The new FS will be optimized
/// for easy programatic navigation and may not be readily usable by humans.
/// 
/// 
#[derive(Debug, Serialize, Deserialize)]
pub struct FsMapper {
    fs: FsNode,
}
impl FsMapper {
    /// Creates an empty mapper.
    pub fn new() -> Self {
        let mut fs = FsNode::new_dir("");
        fs.add_node(FsNode::new_dir("EDFC"), "");
        fs.add_node(FsNode::new_dir("ROMS"), "");
        fs.add_node(FsNode::new_dir("VeriTAS"), "");
        
        Self {
            fs,
        }
    }
    
    /// Loads an existing map from a file.
    pub fn load(path: impl AsRef<Utf8Path>) -> Result<Self, ()> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(path.as_ref())
            .map_err(|_| ())?;
        let fs_mapper = bincode::serde::decode_from_std_read(&mut file, bincode::config::standard()).map_err(|_| ())?;
        
        Ok(fs_mapper)
    }
    
    /// Saves the map as a file.
    pub fn save(&self, path: impl AsRef<Utf8Path>) -> Result<(), ()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path.as_ref())
            .map_err(|_| ())?;
        bincode::serde::encode_into_std_write(&self, &mut file, bincode::config::standard()).map_err(|_| ())?;
        
        Ok(())
    }
    
    pub fn root(&self) -> &FsNode {
        &self.fs
    }
    
    pub fn include_files(&mut self, path: impl Into<Utf8PathBuf>) -> usize {
        let remote = path.into();
        let remote = remote.canonicalize_utf8().ok().unwrap_or_else(|| remote);
        
        if !remote.exists() || remote.file_name().is_none() {
            return 0;
        }
        
        if remote.is_file() {
            let Some(parent) = remote.parent() else { return 0 };
            
            self.include_file(parent, &remote) as usize
        } else if remote.is_dir() {
            let parent = &remote;
            
            let iter = WalkDir::new(&remote)
                .sort_by_file_name()
                .into_iter()
                .filter_entry(|entry| !entry.file_name().to_string_lossy().starts_with('.') && !(entry.file_name().as_bytes() == b"System Volume Information"))
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
                .filter_map(|entry| Utf8PathBuf::from_path_buf(entry.into_path()).ok());
            
            let mut count = 0usize;
            for remote in iter {
                if self.include_file(&parent, remote.strip_prefix(&parent).expect("parent should've been correct")) {
                    count += 1;
                }
                
                if count % 1000 == 0 {
                    println!("processed: {count}");
                }
            }
            
            count
        } else {
            unreachable!()
        }
    }
    
    fn include_file(&mut self, parent: &Utf8Path, local: &Utf8Path) -> bool {
        let remote = parent.join(local);
        assert!(remote.is_file());
        
        if self.fs.path_exists(local) {
            return true;
        }
        
        let Ok(hashes) = HashBundle::from_path(&remote) else { return false };
        
        if hashes.iter().any(|hash| self.fs.contains_hash(hash)) {
            return true;
        }
        
        let remote = RemoteFile {
            path: remote.clone(),
            hashes: hashes.clone(),
        };
        
        let name = local.file_name().expect("filename shouldn't be empty");
        
        let local_parent = local.parent().unwrap_or(".".into());
        
        self.fs.add_node(FsNode::new_file(Some(remote), name, hashes), local_parent)
    }
    
    pub fn register_files(&mut self, path: impl Into<Utf8PathBuf>) -> usize {
        let remote = path.into();
        let remote = remote.canonicalize_utf8().ok().unwrap_or_else(|| remote);
        
        if !remote.exists() || remote.file_name().is_none() {
            return 0;
        }
        
        if remote.is_file() {
            self.register_file(remote) as usize
        } else if remote.is_dir() {
            let iter = WalkDir::new(&remote)
                .sort_by_file_name()
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
                .filter_map(|entry| Utf8PathBuf::from_path_buf(entry.into_path()).ok());
            
            let mut count = 0usize;
            for remote in iter {
                if self.register_file(remote) {
                    count += 1;
                }
            }
            
            count
        } else {
            unreachable!()
        }
    }
    
    fn register_file(&mut self, remote: Utf8PathBuf) -> bool {
        let Ok(hashes) = HashBundle::from_path(&remote) else { return false };
        
        if hashes.iter().any(|hash| self.fs.contains_hash(hash)) {
            return false;
        }
        
        let remote_filename = remote.file_name().expect("empty filenames should've been filtered already");
        if remote_filename.starts_with('.') {
            // both dirs and files that start with `.` do not appear in the everdrive menu
            return false;
        }
        
        let name = if self.fs.contains_filename(remote_filename) {
            let (stem, n, suffix) = filename_parts(remote_filename);
            
            if let Some(n) = n {
                format!("{stem}.{}{suffix}", n + 1)
            } else {
                format!("{stem}.1{suffix}")
            }
        } else {
            remote_filename.to_string()
        };
        
        let remote_file = RemoteFile {
            path: remote,
            hashes: hashes.clone(),
        };
        self.map_file(FsNode::new_file(Some(remote_file), name, hashes));
        
        true
    }
    
    fn map_file(&mut self, file: FsNode) {
        const START_DIR_NAME: &'static str = "_000000";
        
        assert!(matches!(file, FsNode::File {..}), "internal fn should only be provided with FsNode::File");
        
        assert!(self.fs.len() + 1 <= ABS_MAX_DIR_LEN * MAX_PAGES * MAX_PER_PAGE);
        
        if !self.fs.path_exists("VeriTAS") {
            self.fs.add_node(FsNode::new_dir("VeriTAS"), ".");
        }
        
        let FsNode::Dir { children: root_children, .. } = &mut self.fs else { panic!("root node should be a FsNode::Dir") };
        
        root_children.modify("VeriTAS", |veritas_dir| {
            let FsNode::Dir { children, .. } = veritas_dir else { unreachable!() };
            
            let mut ndir_name = START_DIR_NAME.to_string();
            
            if let Some(last_dir) = children.iter().filter(|node| node.is_dir() && node.name().trim_start_matches('_').parse::<usize>().is_ok()).max() {
                if last_dir.len() >= REC_MAX_DIR_LEN {
                    let n: usize = last_dir.name().trim_start_matches('_').parse().expect("should only be using numbered dirs");
                    
                    ndir_name = format!("_{:06}", n + 1)
                } else {
                    ndir_name = last_dir.name().to_string();
                }
            }
            
            veritas_dir.add_node(file, ndir_name);
        });
    }
    
    pub fn copy_to(&self, output: impl AsRef<Utf8Path>) -> bool {
        fn handle_children(current: impl AsRef<Utf8Path>, children: &SortedNodes) {
            let current = current.as_ref();
            for (_, child) in children {
                match child {
                    FsNode::Dir { name, children } => {
                        let dir_path = current.join(name);
                        if !dir_path.is_dir() && let Err(err) = std::fs::create_dir(&dir_path) {
                            println!("{err:?}");
                        }
                        
                        handle_children(dir_path, children);
                    },
                    FsNode::File { remote: Some(remote), .. } => {
                        let to_path = current.join(child.name());
                        if remote.path.is_file() && !to_path.is_file() && let Err(err) = std::fs::copy(&remote.path, to_path) {
                            println!("{err:?}");
                        }
                    }
                    _ => ()
                }
            }
        }
        
        let output = output.as_ref();
        if let Err(err) = std::fs::create_dir_all(output) {
            println!("{err:?}");
            return false;
        }
        
        if let FsNode::Dir { children, .. } = &self.fs {
            handle_children(output, children);
            
            return true;
        }
        
        false
    }
    
    pub fn pretty_print(&self) {
        self.fs.pretty_print();
    }
    
    pub fn find_hash(&self, needles: &[HashBundle]) -> Option<(Utf8PathBuf, &FsNode)> {
        self.fs.find(|node| match node {
            FsNode::File { hashes, .. } => needles.iter().any(|needle| hashes.contains(needle)),
            _ => false,
        })
    }
    
    pub fn find<F: Fn(&FsNode) -> bool>(&self, func: F) -> Option<(Utf8PathBuf, &FsNode)> {
        self.fs.find(func)
    }
}

fn filename_parts(name: &str) -> (&str, Option<usize>, String) {
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    let suffix = if ext.is_empty() {
        format!("")
    } else {
        format!(".{ext}")
    };
    
    if let Some((stem, n)) = stem.rsplit_once('.').map(|(stem, n)| n.parse::<usize>().ok().map(|n| (stem, n))).flatten() {
        (stem, Some(n), suffix)
    } else {
        (stem, None, suffix)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FsNode {
    Dir {
        name: String,
        children: SortedNodes,
    },
    File {
        remote: Option<RemoteFile>,
        name: String,
        hashes: HashSet<HashBundle>,
    }
}
impl PartialOrd for FsNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FsNode {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_dir(), other.is_dir()) {
            (true, true) | (false, false) => {
                self.name().cmp(other.name())
            },
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
        }
    }
}

impl FsNode {
    pub fn new_dir(name: impl Into<String>) -> Self {
        Self::Dir {
            name: name.into(),
            children: SortedNodes::new(),
        }
    }
    
    pub fn new_file<I: IntoIterator<Item = HashBundle>>(remote: Option<RemoteFile>, name: impl Into<String>, hashes: I) -> Self {
        Self::File {
            remote,
            name: name.into(),
            hashes: HashSet::from_iter(hashes),
        }
    }
    
    #[inline(always)]
    pub fn is_dir(&self) -> bool {
        matches!(self, Self::Dir {..})
    }
    
    #[allow(unused)]
    #[inline(always)]
    pub fn is_file(&self) -> bool {
        matches!(self, Self::File {..})
    }
    
    /*
    /// Adds child to directory.
    /// 
    /// Returns `false` if node is a `File` or the child is already in the directory,
    /// otherwise returns `true`.
    pub fn add_child(&mut self, node: FsNode) -> bool {
        match self {
            Self::Dir { children, .. } => {
                children.insert(node).is_some()
            },
            Self::File { .. } => false,
        }
    }*/
    
    /// Adds a node to a given directory, creating parents as necessary.
    /// 
    /// Leading and following `/` in the `path` parameter will be ignored. Thus the `path` will
    /// _always_ be treated as a directory, in which the `node` will be placed.
    /// 
    /// If the effective path is empty (e.g. `""`, `"/"`, or `"."`), and `self` is a directory, the
    /// `node` will be placed in `self`.
    /// 
    /// If the `node`'s name matches a node already in the `path` directory, the old node will be
    /// removed and replaced with the new `node`.
    /// 
    /// # Returns
    /// `true` if the `node` is successfully added.
    pub fn add_node(&mut self, node: FsNode, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        let parts: Vec<&str> = path
            .components()
            .filter_map(|part| if let Utf8Component::Normal(s) = part { Some(s) } else { None })
            .collect();
        
        let FsNode::Dir { children, .. } = self else { return false };
        
        if parts.is_empty() {
            children.insert(node);
            return true;
        }
        
        if !children.contains(parts[0]) {
            children.insert(FsNode::new_dir(parts[0]));
        }
        
        children.modify(parts[0], |child| {
            child.add_node(node, parts[1..].join("/"))
        }).unwrap_or(false)
    }
    
    /// Returns the number of children this node has.
    /// 
    /// If the node is a `File`, then `0` is returned.
    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            Self::Dir { children, .. } => children.len(),
            Self::File { .. } => 0,
        }
    }
    
    #[inline(always)]
    pub fn name(&self) -> &str {
        match self {
            Self::Dir { name, .. } => name.as_str(),
            Self::File { name, .. } => name.as_str(),
        }
    }
    
    pub fn find<F: Fn(&FsNode) -> bool>(&self, func: F) -> Option<(Utf8PathBuf, &FsNode)> {
        self._find(&[], &func).map(|(parts, node)| (parts.join("/").into(), node))
    }
    
    fn _find<'a, F: Fn(&FsNode) -> bool>(&'a self, parts: &[&'a str], func: &F) -> Option<(Vec<&'a str>, &'a FsNode)> {
        let mut parts = parts.to_vec();
        parts.push(self.name());
        
        if func(self) {
            return Some((parts, self));
        }
        
        match self {
            Self::Dir { children, .. } => {
                for (_, child) in children {
                    if let Some(result) = child._find(&parts, func) {
                        return Some(result);
                    }
                }
            },
            Self::File {..} => (),
        }
        
        None
    }
    
    pub fn location(&self, path: impl AsRef<Utf8Path>) -> Option<Vec<usize>> {
        let mut indices = Vec::with_capacity(4);
        
        if self._location(path, &mut indices) {
            Some(indices)
        } else {
            None
        }
    }
    
    fn _location(&self, path: impl AsRef<Utf8Path>, indices: &mut Vec<usize>) -> bool {
        let path = path.as_ref();
        let parts: Vec<&str> = path
            .components()
            .filter_map(|part| if let Utf8Component::Normal(s) = part { Some(s) } else { None })
            .collect();
        
        match self {
            Self::Dir { children, .. } => {
                if let Some((i, child)) = children.iter().enumerate().find(|(_, child)| child.name() == parts[0]) {
                    indices.push(i);
                    
                    if parts.len() == 1 {
                        true
                    } else {
                        child._location(parts[1..].join("/"), indices)
                    }
                } else {
                    false
                }
            },
            Self::File {..} => false,
        }
    }
    
    pub fn path_exists(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        let parts: Vec<&str> = path
            .components()
            .filter_map(|part| if let Utf8Component::Normal(s) = part { Some(s) } else { None })
            .collect();
        
        match self {
            Self::Dir { children, .. } => {
                if let Some(child) = children.iter().find(|child| child.name() == parts[0]) {
                    if parts.len() == 1 {
                        true
                    } else {
                        child.path_exists(parts[1..].join("/"))
                    }
                } else {
                    false
                }
            },
            Self::File {..} => false,
        }
    }
    
    pub fn contains_hash(&self, hash: &HashBundle) -> bool {
        match self {
            Self::Dir { children, .. } => children.iter().any(|child| child.contains_hash(hash)),
            Self::File { hashes, .. } => hashes.contains(hash),
        }
    }
    
    pub fn contains_filename(&self, name: impl AsRef<str>) -> bool {
        let search_name = name.as_ref();
        match self {
            Self::Dir { children, .. } => children.iter().any(|child| child.contains_filename(search_name)),
            Self::File { name, .. } => name == search_name,
        }
    }
    
    pub fn pretty_print(&self) {
        Self::_pretty_print(self, 0);
    }
    
    fn _pretty_print(node: &FsNode, indent: usize) {
        match node {
            FsNode::Dir { children, name, .. } => {
                println!("{:indent$}└── {name}/", "");
                for (_, child) in children {
                    Self::_pretty_print(child, indent + 4);
                }
            },
            FsNode::File { name, remote: Some(remote), .. } => println!("{:indent$}├── {name} ({})", "", remote.path),
            FsNode::File { name, .. } => println!("{:indent$}├── {name}", ""),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use veritas_dump::cache::HashBundle;
    use super::{FsNode, RemoteFile};
    
    #[test]
    fn test_node_ord() {
        assert!(FsNode::new_dir("0") < FsNode::new_dir("a"));
        assert!(FsNode::new_dir("a") < FsNode::new_dir("b"));
        
        assert!(FsNode::new_dir("A") < FsNode::new_dir("_"));
        assert!(FsNode::new_dir("_") < FsNode::new_dir("a"));
        
        assert!(FsNode::new_dir("A") > FsNode::new_file(None, "A", []));
        assert!(FsNode::new_dir("a") > FsNode::new_file(None, "A", []));
    }
    
    #[test]
    fn test_add_file() {
        let mut fs = FsNode::new_dir("");
        
        assert!(fs.add_node(FsNode::new_file(None, "test.txt", []), ""));
        assert!(fs.path_exists("test.txt"));
        
        assert!(fs.add_node(FsNode::new_file(None, "slash.txt", []), "/"));
        assert!(fs.path_exists("slash.txt"));
        
        assert!(fs.add_node(FsNode::new_file(None, "double-slash.txt", []), "//"));
        assert!(fs.path_exists("double-slash.txt"));
        
        assert!(fs.add_node(FsNode::new_file(None, "period.txt", []), "."));
        assert!(fs.path_exists("period.txt"));
        
        assert!(fs.add_node({
            let mut dir = FsNode::new_dir("aaa");
            dir.add_node(FsNode::new_file(None, "atest.txt", []), "");
            
            dir
        }, "/"));
        assert!(fs.path_exists("aaa/atest.txt"));
        
        assert!(fs.add_node(FsNode::new_file(None, "mkdir.txt", []), "/bbb/"));
        assert!(fs.path_exists("bbb/mkdir.txt"));
        
        assert!(fs.add_node(FsNode::new_file(None, "double-nested.bin", []), "ccc/nested/"));
        assert!(fs.path_exists("ccc/nested/double-nested.bin"));
    }
    
    #[test]
    fn test_add_dir() {
        let mut fs = FsNode::new_dir("");
        
        assert!(fs.add_node(FsNode::new_dir("aaa"), ""));
        assert!(fs.path_exists("aaa"));
        assert!(fs.path_exists("aaa/"));
        assert!(fs.path_exists("/aaa"));
        assert!(fs.path_exists("/aaa/"));
        
        assert!(fs.add_node(FsNode::new_dir("zzz"), "a/b/c/d/e/f/false.txt"));
        assert!(fs.path_exists("/a/b/c/d/e/f/false.txt/zzz"));
    }
    
    #[test]
    fn test_find() {
        let hash = HashBundle::new(&[1, 2, 3, 4, 5, 69]);
        
        let mut fs = FsNode::new_dir("");
        fs.add_node(FsNode::new_file(None, "test.txt", []), "aaa/");
        fs.add_node(FsNode::new_dir("bbb"), ".");
        fs.add_node(FsNode::new_dir("ccc"), ".");
        fs.add_node(FsNode::new_file(Some(RemoteFile { path: "".into(), hashes: HashSet::new() }), "test.txt", []), ".");
        fs.add_node(FsNode::new_file(None, "search.bin", [hash.clone()]), "/a/b/c/d/false.txt/zzz/");
        
        assert!(fs.find(|node| node.name() == "ccc").is_some());
        assert!(fs.find(|node| if node.name() == "test.txt" && let FsNode::File { remote, .. } = node && remote.is_some() { true } else { false }).is_some());
        assert!(fs.find(|node| if let FsNode::File { hashes, .. } = node && hashes.contains(&hash) { true } else { false }).is_some());
    }
    
    #[test]
    fn test_location() {
        let mut fs = FsNode::new_dir("");
        fs.add_node(FsNode::new_file(None, "0.txt", []), "a/b/c/d/e/f/");
        fs.add_node(FsNode::new_file(None, "test.txt", []), "a/b/c/d/e/f/");
        fs.add_node(FsNode::new_file(None, "1.txt", []), "a/b/c");
        fs.add_node(FsNode::new_file(None, "2.txt", []), "a/b/c");
        fs.add_node(FsNode::new_file(None, "3.txt", []), "a/b/c");
        fs.pretty_print();
        
        assert_eq!(fs.location("a/b/c/fake"), None);
        assert_eq!(fs.location("a/b/c/test.txt"), None);
        
        assert_eq!(fs.location("/a/b/c/d/e/f/test.txt"), Some(vec![0, 0, 0, 0, 0, 0, 1]));
        assert_eq!(fs.location("/a/b/c/d/"), Some(vec![0, 0, 0, 0]));
        assert_eq!(fs.location("/a/b/c/1.txt"), Some(vec![0, 0, 0, 1]));
        assert_eq!(fs.location("/a/b/c/2.txt"), Some(vec![0, 0, 0, 2]));
        assert_eq!(fs.location("/a/b/c/3.txt"), Some(vec![0, 0, 0, 3]));
    }
}