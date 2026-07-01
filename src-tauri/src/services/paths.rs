use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 标记文件名：记录当前生效的数据目录绝对路径。
/// 固定存放在系统 app_data_dir 下（小文件，C 盘占用可忽略）。
const MARKER_FILENAME: &str = "data_dir.txt";

/// 解析生效的数据目录。
///
/// 优先级：
/// 1. 标记文件已记录的路径（用户曾更改过 / 上次已确定）→ 直接用
/// 2. exe 同级目录下的 `data/`，且可写 → 用它（绿色便携，跟随安装位置）
/// 3. 系统默认 `app_data_dir`（兜底，如装在 Program Files 无写权限时）
pub fn resolve_data_dir(fallback_dir: &Path) -> PathBuf {
    let marker_path = fallback_dir.join(MARKER_FILENAME);

    // 1. 标记文件优先
    if let Ok(recorded) = fs::read_to_string(&marker_path) {
        let recorded = recorded.trim();
        let candidate = PathBuf::from(recorded);
        if candidate.is_dir() && is_writable(&candidate) {
            return candidate;
        }
        // 标记失效（目录被删/无权限）→ 继续探测，后续会重写标记
    }

    // 2. exe 同级 data 目录
    if let Some(exe_data_dir) = exe_sibling_data_dir() {
        if ensure_dir(&exe_data_dir) && is_writable(&exe_data_dir) {
            persist_marker(&marker_path, &exe_data_dir);
            return exe_data_dir;
        }
    }

    // 3. 兜底：系统默认目录
    if ensure_dir(fallback_dir) {
        persist_marker(&marker_path, fallback_dir);
    }
    fallback_dir.to_path_buf()
}

/// 更改数据目录：迁移旧数据到新目录，更新标记。
///
/// 返回新目录路径。调用方需注意：数据库连接此时不应持有旧库文件。
pub fn change_data_dir(fallback_dir: &Path, new_dir: &Path) -> Result<PathBuf, String> {
    // 新目录必须可创建且可写
    if !ensure_dir(new_dir) {
        return Err(format!("无法创建目录: {}", new_dir.display()));
    }
    if !is_writable(new_dir) {
        return Err(format!("目录不可写: {}", new_dir.display()));
    }

    let marker_path = fallback_dir.join(MARKER_FILENAME);
    let old_dir = fs::read_to_string(&marker_path)
        .ok()
        .and_then(|s| {
            let p = PathBuf::from(s.trim());
            if p.is_dir() { Some(p) } else { None }
        })
        .unwrap_or_else(|| resolve_data_dir(fallback_dir));

    // 迁移内容（db + cache/*.gz），逐项复制，已存在的目标跳过
    if old_dir != new_dir {
        migrate_dir(&old_dir, new_dir)?;
    }

    persist_marker(&marker_path, new_dir);
    Ok(new_dir.to_path_buf())
}

/// exe 同级目录下的 data 子目录
fn exe_sibling_data_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    Some(exe_dir.join("data"))
}

/// 确保目录存在，返回是否最终可用
fn ensure_dir(dir: &Path) -> bool {
    fs::create_dir_all(dir).is_ok() && dir.is_dir()
}

/// 检测目录是否可写（创建临时文件验证）
fn is_writable(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(format!(".write_probe_{}", std::process::id()));
    let writable = fs::File::create(&probe).and_then(|mut f| f.write_all(b"ok")).is_ok();
    let _ = fs::remove_file(&probe);
    writable
}

/// 写入标记文件（失败不致命，下次会重新探测）
fn persist_marker(marker_path: &Path, data_dir: &Path) {
    if let Some(parent) = marker_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut f) = fs::File::create(marker_path) {
        let _ = f.write_all(data_dir.to_string_lossy().as_bytes());
    }
}

/// 递归迁移目录内容（复制，不删除源）
fn migrate_dir(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(src).map_err(|e| format!("读取源目录失败: {e}"))? {
        let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
        let from = entry.path();
        let name = entry.file_name();
        let to = dst.join(&name);
        let ft = entry.file_type().map_err(|e| format!("读取类型失败: {e}"))?;
        if ft.is_dir() {
            fs::create_dir_all(&to).map_err(|e| format!("创建子目录失败: {e}"))?;
            migrate_dir(&from, &to)?;
        } else {
            // 目标已存在则跳过，避免覆盖用户已有数据
            if !to.exists() {
                fs::copy(&from, &to).map_err(|e| format!("复制文件失败: {e}"))?;
            }
        }
    }
    Ok(())
}

/// 计算目录总大小（递归，字节）
pub fn dir_size(dir: &Path) -> u64 {
    fn inner(dir: &Path) -> u64 {
        let mut total = 0;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        total += inner(&path);
                    } else if ft.is_file() {
                        total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                    }
                }
            }
        }
        total
    }
    if dir.is_dir() { inner(dir) } else { 0 }
}

/// 清空 cache 子目录内容（保留目录本身）
pub fn clear_cache(data_dir: &Path) -> Result<(), String> {
    let cache_dir = data_dir.join("cache");
    if !cache_dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(&cache_dir).map_err(|e| format!("读取缓存目录失败: {e}"))? {
        let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| format!("删除目录失败: {e}"))?;
        } else {
            fs::remove_file(&path).map_err(|e| format!("删除文件失败: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolve_uses_marker_when_valid() {
        let tmp = tempdir().unwrap();
        let fallback = tmp.path().to_path_buf();
        let recorded = fallback.join("my_data");
        fs::create_dir_all(&recorded).unwrap();

        let marker = fallback.join(MARKER_FILENAME);
        fs::write(&marker, recorded.to_str().unwrap()).unwrap();

        let resolved = resolve_data_dir(&fallback);
        assert_eq!(resolved, recorded);
    }

    #[test]
    fn resolve_falls_back_when_marker_invalid() {
        let tmp = tempdir().unwrap();
        let fallback = tmp.path().to_path_buf();
        let marker = fallback.join(MARKER_FILENAME);
        fs::write(&marker, "/nonexistent/path/xyz").unwrap();

        let resolved = resolve_data_dir(&fallback);
        // 应该是 exe 同级（CI 环境下可能是临时目录）或 fallback，总之是可写目录
        assert!(resolved.is_dir());
        assert!(is_writable(&resolved));
    }

    #[test]
    fn change_dir_migrates_files() {
        let tmp = tempdir().unwrap();
        let fallback = tmp.path().to_path_buf();
        let old = fallback.join("old");
        let new = fallback.join("new");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("lingjian.db"), b"db content").unwrap();
        fs::create_dir_all(old.join("cache")).unwrap();
        fs::write(old.join("cache").join("abc.gz"), b"gz").unwrap();

        // 先标记 old 为当前目录
        let marker = fallback.join(MARKER_FILENAME);
        fs::write(&marker, old.to_str().unwrap()).unwrap();

        let result = change_data_dir(&fallback, &new).unwrap();
        assert_eq!(result, new);
        assert!(new.join("lingjian.db").exists());
        assert!(new.join("cache").join("abc.gz").exists());
    }

    #[test]
    fn dir_size_counts_files() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("a.bin"), vec![0u8; 100]).unwrap();
        fs::create_dir_all(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("sub").join("b.bin"), vec![0u8; 50]).unwrap();
        assert_eq!(dir_size(tmp.path()), 150);
    }

    #[test]
    fn clear_cache_removes_contents() {
        let tmp = tempdir().unwrap();
        let cache = tmp.path().join("cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("a.gz"), b"x").unwrap();
        fs::write(cache.join("b.gz"), b"y").unwrap();

        clear_cache(tmp.path()).unwrap();
        assert!(cache.is_dir());
        assert!(fs::read_dir(&cache).unwrap().count() == 0);
    }
}
