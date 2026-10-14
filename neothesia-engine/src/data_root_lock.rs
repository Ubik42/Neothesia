use std::{fs::{File, OpenOptions}, path::{Path, PathBuf}};

/// An exclusive lease on one Neothesia data directory. Keep this value alive
/// for as long as any part of the directory is in use.
pub struct DataRootLock {
    root: PathBuf,
    _file: File,
}

impl DataRootLock {
    pub fn acquire(root: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(root)
            .map_err(|error| format!("无法创建用户资料目录 {}：{error}", root.display()))?;
        let root = root.canonicalize()
            .map_err(|error| format!("无法读取用户资料目录 {}：{error}", root.display()))?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(root.join("desktop.lock"))
            .map_err(|error| format!("无法打开用户资料锁 {}：{error}", root.display()))?;
        file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => format!(
                    "用户资料目录正在被另一个 Neothesia 实例使用：{}。请关闭占用该目录的桌面版或 Web 服务，或选择其他 NEOTHESIA_DATA_ROOT。",
                    root.display()
                ),
            std::fs::TryLockError::Error(error) => {
                format!("无法独占用户资料目录 {}：{error}", root.display())
            }
        })?;
        Ok(Self { root, _file: file })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn matches(&self, root: &Path) -> Result<(), String> {
        let requested = root.canonicalize()
            .map_err(|error| format!("无法读取用户资料目录 {}：{error}", root.display()))?;
        if requested != self.root {
            return Err(format!(
                "引擎使用的用户资料目录与已锁定目录不一致：{}；已锁定：{}",
                requested.display(), self.root.display()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::DataRootLock;

    #[test]
    fn exclusive_per_canonical_directory_and_released_on_drop() {
        let base = std::env::temp_dir().join(format!(
            "neothesia-data-root-lock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let first = base.join("first");
        let second = base.join("second");
        let lock = DataRootLock::acquire(&first).unwrap();
        assert!(DataRootLock::acquire(&first).err().unwrap().contains("正在被另一个"));
        let other = DataRootLock::acquire(&second).unwrap();
        assert!(lock.matches(&second).is_err());
        assert!(lock.matches(&first.join(".")).is_ok());
        drop(lock);
        let again = DataRootLock::acquire(&first).unwrap();
        drop(again);
        drop(other);
        std::fs::remove_dir_all(base).unwrap();
    }
}
