use std::collections::HashSet;
use bincode::{Decode, Encode};
use camino::{Utf8Path, Utf8PathBuf};
use md5::Digest;
use walkdir::WalkDir;
/*
#[derive(Debug, Clone, Default, Decode, Encode)]
struct KeyedData {
    ids: BTreeSet<usize>,
    entries: HashMap<HashBundle, usize>,
    paths: HashMap<String, usize>,
}
impl KeyedData {
    pub fn insert(&mut self, path: String, entry: HashBundle) {
        match (self.paths.get(&path), self.entries.get(&entry)) {
            (None, None) => {
                let id = self.ids.last().unwrap_or(&0) + 1;
                
                self.ids.insert(id);
                self.paths.insert(path, id);
                self.entries.insert(entry, id);
            },
            
            (Some(id), None) => {
                // any existing hashes must be invalid
                self.entries.retain(|_, eid| eid != id);
                
                self.entries.insert(entry, *id);
            },
            
            (None, Some(id)) => {
                self.paths.insert(path, *id);
            },
            
            (Some(pid), Some(eid)) => {
                if pid != eid {
                    unimplemented!("don't know what to do with this, pid ({pid}) != eid ({eid}), path: {path}, entry: {entry:02X?}");
                }
            }
        }
    }
    
    pub fn get_entries(&self, path: &str) -> Vec<HashBundle> {
        let Some(id) = self.paths.get(path) else { return vec![] };
        
        self.entries
            .iter()
            .filter_map(|(entry, eid)| if eid == id { Some(entry.clone()) } else { None })
            .collect()
    }
    
    pub fn get_paths(&self, entry: &HashBundle) -> Vec<String> {
        let Some(id) = self.entries.get(entry) else { return vec![] };
        
        self.paths
            .iter()
            .filter_map(|(path, pid)| if pid == id { Some(path.clone()) } else { None })
            .collect()
    }
    
    pub fn validate_paths(&mut self) {
        let mut invalid_ids = vec![];
        
        self.paths.iter().for_each(|(path, id)| if !Utf8Path::new(path).is_file() { invalid_ids.push(*id) });
        
        for id in invalid_ids {
            self.paths.retain(|_, pid| *pid != id);
            self.entries.retain(|_, eid| *eid != id);
        }
    }
    
    pub fn check_integrity(&self) -> (Vec<usize>, Vec<String>, Vec<HashBundle>) {
        let mut missing_ids = vec![];
        
        let mut floating_paths = vec![];
        for (path, pid) in &self.paths {
            if !self.entries.values().any(|id| id == pid) {
                floating_paths.push(path.clone());
            }
            
            if !self.ids.contains(pid) {
                missing_ids.push(*pid);
            }
        }
        
        let mut floating_entries = vec![];
        for (entry, eid) in &self.entries {
            if !self.paths.values().any(|id| id == eid) {
                floating_entries.push(entry.clone());
            }
            
            if !self.ids.contains(eid) {
                missing_ids.push(*eid);
            }
        }
        
        (missing_ids, floating_paths, floating_entries)
    }
}
*/
#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct UniqueItem {
    pub paths: HashSet<String>,
    pub hashes: HashSet<HashBundle>,
}

// Is this the most efficient data structure? No, probably not. But even with 50,000 elements,
// worst case lookups are still plenty fast and when this is serialized it takes up very little space.
#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct Cache {
    items: Vec<UniqueItem>
}
impl Cache {
    /// Creates a new empty cache.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Refreshes the cache by removing missing files and including new files.
    /// 
    /// Hashes belonging to files that no longer exist will be removed.
    /// 
    /// Optionally, a path to a directory (recursive) or a file can be provided. These files
    /// will be added to the cache _if the file path isn't already in the cache._
    /// 
    /// Files that cannot be read (e.g. lack of permissions) will be silently ignored.
    /// 
    /// Use [`Cache::force_rehash`] to force the cache to rehash the given files.
    pub fn refresh(&mut self, path: Option<&str>) {
        let mut recalc_paths = vec![];
        self.items.retain_mut(|item| {
            item.paths.retain(|path| Utf8Path::new(&path).is_file());
            
            if !item.paths.is_empty() && item.hashes.is_empty() {
                recalc_paths.extend(item.paths.iter().cloned());
            }
            
            !item.paths.is_empty() && !item.hashes.is_empty()
        });
        
        // shouldn't be possible, but just in case an item has zero hashes the paths should be recalced
        for path in recalc_paths {
            let Ok(hashes) = HashBundle::from_path(&path) else { continue };
            if !hashes.is_empty() {
                self.items.push(UniqueItem {
                    paths: HashSet::from([path]),
                    hashes,
                });
            }
        }
        
        let mut add_count = 0usize;
        if let Some(path) = path {
            for path in Self::walked_dir(path) {
                if !self.items.iter().any(|item| item.paths.iter().any(|p| p == path.as_str())) {
                    let Ok(hashes) = HashBundle::from_path(&path) else { continue };
                    if !hashes.is_empty() {
                        add_count += 1;
                        self.items.push(UniqueItem {
                            paths: HashSet::from([path.to_string()]),
                            hashes,
                        });
                    }
                }
            }
        }
        
        if add_count > 0 {
            self.merge_duplicates();
        }
    }
    
    /// Computes the hashes for a file or directory (recursively).
    /// 
    /// If a file already exists in the cache, its hashes will
    /// 
    /// Previously uncached files will also be added.
    pub fn force_rehash<P: AsRef<Utf8Path>>(&mut self, path: P) {
        for path in Self::walked_dir(path) {
            let mut to_calc = HashSet::from([path.to_string()]);
            self.items.retain(|item| {
                if item.paths.iter().any(|p| p == path.as_str()) {
                    to_calc.extend(item.paths.iter().cloned());
                    
                    return false;
                }
                
                true
            });
            
            for path in to_calc {
                let Ok(hashes) = HashBundle::from_path(&path) else { continue };
                if !hashes.is_empty() {
                    self.items.push(UniqueItem {
                        paths: HashSet::from([path]),
                        hashes,
                    });
                }
            }
        }
        
        self.merge_duplicates();
    }
    
    pub fn find_paths<F: Fn(&HashBundle) -> bool>(&self, predicate: F) -> HashSet<String> {
        if let Some(item) = self.items.iter().find(|item| item.hashes.iter().any(|hash| predicate(hash))) {
            item.paths.clone()
        } else {
            HashSet::new()
        }
    }
    
    pub fn find_hashes(&self, path: &str) -> HashSet<HashBundle> {
        self.items.iter()
            .filter(|item| item.paths.iter().any(|p| p == path))
            .map(|item| item.hashes.clone())
            .flatten()
            .collect()
    }
    
    pub fn encode(&self) -> Vec<u8> {
        bincode::encode_to_vec(self, bincode::config::standard()).expect("bincode failed to encode Cache")
    }
    
    pub fn decode(data: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        bincode::decode_from_slice(data, bincode::config::standard()).map(|(cache, _)| cache)
    }
    
    fn walked_dir<P: AsRef<Utf8Path>>(path: P) -> impl Iterator<Item = Utf8PathBuf> {
        WalkDir::new(path.as_ref())
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter_map(|e| Utf8Path::from_path(e.path())?.canonicalize_utf8().ok())
    }
    
    fn merge_duplicates(&mut self) {
        // scans item list from right to left, merging the paths of duplicate items into the left-hand item
        'rhs_loop: for r in (0..self.items.len()).rev() {
            for l in (0..r).rev() {
                if self.items[r].hashes == self.items[l].hashes {
                    let paths = self.items[r].paths.clone();
                    
                    self.items[l].paths.extend(paths.into_iter());
                    
                    self.items.remove(r);
                    
                    continue 'rhs_loop;
                }
            }
        }
    }
}

/// A bundle of hashes for some data using different hash algorithms.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Decode, Encode)]
pub struct HashBundle {
    pub sha1: [u8; 20],
    pub md5: [u8; 16],
}
impl HashBundle {
    /// Computes hashes for a slice of bytes.
    pub fn new(data: &[u8]) -> Self {
        Self {
            sha1: sha1::Sha1::digest(data).into(),
            md5: md5::Md5::digest(data).into(),
        }
    }
    
    /// Computes hashes for a file.
    /// 
    /// Some files, such as NES ROMs, may produce multiple hashes which represent different variations
    /// of the same file (e.g. different header metadata)
    pub fn from_path<P: AsRef<Utf8Path>>(path: P) -> Result<HashSet<Self>, std::io::Error> {
        let path = path.as_ref();
        
        let data = std::fs::read(path)?;
        
        let mut entries = HashSet::with_capacity(1);
        entries.insert(Self::new(&data));
        //println!("{}", hex::encode_upper(Self::new(&data).sha1));
        
        let ext = path.extension().unwrap_or("");
        if data.len() > 16 && (data[0..=3] == [0x4E, 0x45, 0x53, 0x1A] || ext.eq_ignore_ascii_case("nes")) {
            entries.reserve(4);
            // NES ROMs are known to have many header variations despite being the same game.
            // The common permutations are covered here.
            
            entries.insert(Self::new(&data[16..]));
            //println!("{}", hex::encode_upper(Self::new(&data[16..]).md5));
            
            let mut data = data.clone();
            data[8..=15].fill(0);
            entries.insert(Self::new(&data));
            
            data[8] = 0x01;
            entries.insert(Self::new(&data));
            
            data[7..=15].fill(0);
            entries.insert(Self::new(&data));
        }
        
        Ok(entries)
    }
    
    /// Checks if this hash matches a file.
    /// 
    /// Returns false if any IO errors are encountered, or if the file's hashes don't match `self`.
    pub fn validate<P: AsRef<Utf8Path>>(&self, path: P) -> bool {
        let Ok(hashes) = Self::from_path(path) else { return false };
        
        hashes.contains(self)
    }
}