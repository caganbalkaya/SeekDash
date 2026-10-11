use crate::models::{ClipboardItem, IndexedItem, ResultKind, SearchResult};
#[cfg(test)]
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        match Self::open_once(path) {
            Ok(database) => Ok(database),
            Err(error) if is_corrupt(&error) && path.exists() => {
                let timestamp = unix_now();
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("glass.db");
                let backup = path.with_file_name(format!("{file_name}.corrupt-{timestamp}"));
                std::fs::rename(path, &backup).map_err(|_| error)?;
                let _ = std::fs::rename(
                    path.with_extension("db-wal"),
                    std::path::PathBuf::from(format!("{}-wal", backup.display())),
                );
                let _ = std::fs::rename(
                    path.with_extension("db-shm"),
                    std::path::PathBuf::from(format!("{}-shm", backup.display())),
                );
                Self::open_once(path)
            }
            Err(error) => Err(error),
        }
    }

    fn open_once(path: &Path) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        connection.pragma_update(None, "secure_delete", "ON")?;
        connection.execute_batch(SCHEMA)?;
        let _ = connection.execute(
            "ALTER TABLE files ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0",
            [],
        );
        if connection
            .execute(
                "INSERT INTO files_fts(files_fts,rank) VALUES('integrity-check',1)",
                [],
            )
            .is_err()
        {
            connection.execute("INSERT INTO files_fts(files_fts) VALUES('rebuild')", [])?;
        }
        Ok(Self { connection })
    }

    pub fn replace_all(&mut self, items: &[IndexedItem]) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;
        let usage: std::collections::HashMap<String, (i64, i64, i64)> = {
            let mut query =
                transaction.prepare("SELECT path,open_count,last_opened,favorite FROM files")?;
            let rows = query
                .query_map([], |row| {
                    Ok((row.get(0)?, (row.get(1)?, row.get(2)?, row.get(3)?)))
                })?
                .filter_map(Result::ok)
                .collect();
            rows
        };
        transaction.execute("DELETE FROM files", [])?;
        {
            let mut statement = transaction.prepare("INSERT INTO files(name, normalized_name, path, parent, extension, kind, modified, open_count, last_opened, favorite) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(path) DO UPDATE SET name=excluded.name,normalized_name=excluded.normalized_name,parent=excluded.parent,extension=excluded.extension,kind=excluded.kind,modified=excluded.modified")?;
            for item in items {
                let (open_count, last_opened, favorite) =
                    usage.get(&item.path).copied().unwrap_or((0, 0, 0));
                statement.execute(params![
                    item.name,
                    item.normalized_name,
                    item.path,
                    item.parent,
                    item.extension,
                    item.kind.as_str(),
                    item.modified,
                    open_count,
                    last_opened,
                    favorite
                ])?;
            }
        }
        transaction.commit()
    }

    #[cfg(test)]
    pub fn upsert(&self, item: &IndexedItem) -> rusqlite::Result<()> {
        self.connection.execute(
            "DELETE FROM files WHERE path<>?1 AND lower(path)=lower(?1)",
            [&item.path],
        )?;
        self.connection.execute("INSERT INTO files(name,normalized_name,path,parent,extension,kind,modified) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(path) DO UPDATE SET name=excluded.name,normalized_name=excluded.normalized_name,parent=excluded.parent,extension=excluded.extension,kind=excluded.kind,modified=excluded.modified",
            params![item.name,item.normalized_name,item.path,item.parent,item.extension,item.kind.as_str(),item.modified])?;
        Ok(())
    }

    #[cfg(test)]
    pub fn remove_path(&self, path: &str) -> rusqlite::Result<()> {
        self.connection.execute(
            "DELETE FROM files WHERE lower(path)=lower(?1) OR lower(substr(path,1,length(?1)+1))=lower(?1 || '\\')",
            [path],
        )?;
        Ok(())
    }

    pub fn apply_index_changes(
        &mut self,
        upserts: &[IndexedItem],
        removals: &[String],
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;
        {
            let mut remove = transaction.prepare("DELETE FROM files WHERE lower(path)=lower(?1) OR lower(substr(path,1,length(?1)+1))=lower(?1 || '\\')")?;
            for path in removals {
                remove.execute([path])?;
            }
        }
        {
            let mut remove_case_alias = transaction
                .prepare("DELETE FROM files WHERE path<>?1 AND lower(path)=lower(?1)")?;
            let mut upsert = transaction.prepare("INSERT INTO files(name,normalized_name,path,parent,extension,kind,modified) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(path) DO UPDATE SET name=excluded.name,normalized_name=excluded.normalized_name,parent=excluded.parent,extension=excluded.extension,kind=excluded.kind,modified=excluded.modified")?;
            for item in upserts {
                remove_case_alias.execute([&item.path])?;
                upsert.execute(params![
                    item.name,
                    item.normalized_name,
                    item.path,
                    item.parent,
                    item.extension,
                    item.kind.as_str(),
                    item.modified
                ])?;
            }
        }
        transaction.commit()
    }

    pub fn search_candidates(
        &self,
        normalized: &str,
        limit: usize,
    ) -> rusqlite::Result<Vec<SearchResult>> {
        if normalized.is_empty() {
            return self.recent(limit);
        }
        let fts = normalized
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.replace('"', ""))
            .filter(|s| !s.is_empty())
            .map(|s| format!("\"{s}\"*"))
            .collect::<Vec<_>>()
            .join(" ");
        if fts.is_empty() {
            return Ok(Vec::new());
        }
        let mut statement = self.connection.prepare("SELECT f.id,f.name,f.path,f.parent,f.kind,f.open_count,f.last_opened,f.favorite,bm25(files_fts,5.0,2.5,1.0,0.5,0.3) FROM files_fts JOIN files f ON f.id=files_fts.rowid WHERE files_fts MATCH ?1 ORDER BY 9 LIMIT ?2")?;
        let mapped = statement.query_map(params![fts, limit as i64], |row| {
            Ok(SearchResult {
                id: row.get::<_, i64>(0)?.to_string(),
                title: row.get(1)?,
                subtitle: row.get(3)?,
                path: row.get(2)?,
                kind: ResultKind::from_db(&row.get::<_, String>(4)?),
                score: -(row.get::<_, f64>(8)?)
                    + usage_score(row.get::<_, i64>(5)?, row.get::<_, i64>(6)?),
                icon: None,
                favorite: row.get::<_, i64>(7)? != 0,
            })
        })?;
        mapped.collect()
    }

    pub fn fuzzy_candidates(
        &self,
        needle: &str,
        limit: usize,
    ) -> rusqlite::Result<Vec<SearchResult>> {
        let chars: Vec<_> = needle.chars().collect();
        let first_anchor = chars.get(1).or(chars.first()).copied().unwrap_or_default();
        let second_anchor = chars.last().copied().unwrap_or(first_anchor);
        let anchors = [format!("%{first_anchor}%"), format!("%{second_anchor}%")];
        let mut statement = self.connection.prepare("SELECT id,name,path,parent,kind,normalized_name,open_count,last_opened,favorite FROM files WHERE normalized_name LIKE ?1 AND normalized_name LIKE ?2 ORDER BY favorite DESC,open_count DESC,last_opened DESC LIMIT 300")?;
        let rows = statement.query_map(anchors, |row| {
            let candidate: String = row.get(5)?;
            let similarity = strsim::jaro_winkler(needle, &candidate);
            Ok((
                similarity,
                SearchResult {
                    id: row.get::<_, i64>(0)?.to_string(),
                    title: row.get(1)?,
                    path: row.get(2)?,
                    subtitle: row.get(3)?,
                    kind: ResultKind::from_db(&row.get::<_, String>(4)?),
                    score: similarity * 70.0
                        + usage_score(row.get::<_, i64>(6)?, row.get::<_, i64>(7)?),
                    icon: None,
                    favorite: row.get::<_, i64>(8)? != 0,
                },
            ))
        })?;
        let mut values: Vec<_> = rows
            .filter_map(Result::ok)
            .filter(|(s, _)| *s > 0.64)
            .collect();
        values.sort_by(|a, b| b.0.total_cmp(&a.0));
        Ok(values.into_iter().take(limit).map(|(_, r)| r).collect())
    }

    fn recent(&self, limit: usize) -> rusqlite::Result<Vec<SearchResult>> {
        let mut statement = self.connection.prepare("SELECT id,name,path,parent,kind,(open_count*10.0),favorite FROM files WHERE favorite=1 OR open_count>0 ORDER BY favorite DESC,last_opened DESC,open_count DESC LIMIT ?1")?;
        let rows = statement.query_map([limit as i64], |row| {
            Ok(SearchResult {
                id: row.get::<_, i64>(0)?.to_string(),
                title: row.get(1)?,
                path: row.get(2)?,
                subtitle: row.get(3)?,
                kind: ResultKind::from_db(&row.get::<_, String>(4)?),
                score: row.get(5)?,
                icon: None,
                favorite: row.get::<_, i64>(6)? != 0,
            })
        })?;
        rows.collect()
    }

    pub fn record_open(&self, path: &str) -> rusqlite::Result<()> {
        self.connection.execute(
            "UPDATE files SET open_count=open_count+1,last_opened=unixepoch() WHERE path=?1",
            [path],
        )?;
        Ok(())
    }
    pub fn set_favorite(&self, path: &str, favorite: bool) -> rusqlite::Result<()> {
        self.connection.execute(
            "UPDATE files SET favorite=?2 WHERE path=?1",
            params![path, favorite as i64],
        )?;
        Ok(())
    }
    pub fn clear_history(&self) -> rusqlite::Result<()> {
        self.connection
            .execute("UPDATE files SET open_count=0,last_opened=0", [])?;
        Ok(())
    }
    pub fn count(&self) -> rusqlite::Result<u64> {
        self.connection
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
    }
    pub fn load_clipboard_history(&self, limit: usize) -> rusqlite::Result<Vec<ClipboardItem>> {
        let mut statement = self.connection.prepare(
            "SELECT id,text,copied_at FROM clipboard_history ORDER BY copied_at DESC,id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit.min(100) as i64], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                text: row.get(1)?,
                copied_at: row.get(2)?,
            })
        })?;
        rows.collect()
    }
    pub fn save_clipboard_item(&self, item: &ClipboardItem, limit: usize) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO clipboard_history(id,text,copied_at) VALUES(?1,?2,?3)",
            params![item.id, item.text, item.copied_at],
        )?;
        self.connection.execute(
            "DELETE FROM clipboard_history WHERE id NOT IN (SELECT id FROM clipboard_history ORDER BY copied_at DESC,id DESC LIMIT ?1)",
            [limit.min(100) as i64],
        )?;
        Ok(())
    }
    pub fn delete_clipboard_item(&self, id: i64) -> rusqlite::Result<()> {
        self.connection
            .execute("DELETE FROM clipboard_history WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn clear_clipboard_history(&self) -> rusqlite::Result<()> {
        self.connection
            .execute("DELETE FROM clipboard_history", [])?;
        let _ = self
            .connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        Ok(())
    }
    #[cfg(test)]
    pub fn contains_path(&self, path: &str) -> rusqlite::Result<bool> {
        Ok(self
            .connection
            .query_row("SELECT 1 FROM files WHERE path=?1", [path], |_| Ok(true))
            .optional()?
            .unwrap_or(false))
    }
}

fn usage_score(open_count: i64, last_opened: i64) -> f64 {
    let frequency = ((open_count.max(0) + 1) as f64).ln() * 8.0;
    let age = unix_now().saturating_sub(last_opened);
    let recency = if last_opened <= 0 {
        0.0
    } else if age < 86_400 {
        24.0
    } else if age < 604_800 {
        12.0
    } else if age < 2_592_000 {
        5.0
    } else {
        0.0
    };
    frequency + recency
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn is_corrupt(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(inner, _)
            if matches!(inner.code, rusqlite::ffi::ErrorCode::DatabaseCorrupt | rusqlite::ffi::ErrorCode::NotADatabase)
    )
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS files(id INTEGER PRIMARY KEY,name TEXT NOT NULL,normalized_name TEXT NOT NULL,path TEXT NOT NULL UNIQUE,parent TEXT NOT NULL,extension TEXT NOT NULL,kind TEXT NOT NULL,modified INTEGER NOT NULL DEFAULT 0,open_count INTEGER NOT NULL DEFAULT 0,last_opened INTEGER NOT NULL DEFAULT 0,favorite INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS idx_files_normalized ON files(normalized_name);
CREATE INDEX IF NOT EXISTS idx_files_recent ON files(last_opened DESC,open_count DESC);
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(name,normalized_name,path,parent,extension,content='files',content_rowid='id',tokenize='unicode61 remove_diacritics 2');
CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files BEGIN INSERT INTO files_fts(rowid,name,normalized_name,path,parent,extension) VALUES(new.id,new.name,new.normalized_name,new.path,new.parent,new.extension); END;
CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files BEGIN INSERT INTO files_fts(files_fts,rowid,name,normalized_name,path,parent,extension) VALUES('delete',old.id,old.name,old.normalized_name,old.path,old.parent,old.extension); END;
CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE OF name,normalized_name,path,parent,extension ON files BEGIN INSERT INTO files_fts(files_fts,rowid,name,normalized_name,path,parent,extension) VALUES('delete',old.id,old.name,old.normalized_name,old.path,old.parent,old.extension); INSERT INTO files_fts(rowid,name,normalized_name,path,parent,extension) VALUES(new.id,new.name,new.normalized_name,new.path,new.parent,new.extension); END;
CREATE TABLE IF NOT EXISTS clipboard_history(id INTEGER PRIMARY KEY,text TEXT NOT NULL,copied_at INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS idx_clipboard_history_time ON clipboard_history(copied_at DESC);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::normalize;
    fn item(name: &str, path: &str) -> IndexedItem {
        IndexedItem {
            name: name.into(),
            normalized_name: normalize(name),
            path: path.into(),
            parent: r"C:\Test".into(),
            extension: "txt".into(),
            kind: ResultKind::File,
            modified: 1,
        }
    }

    #[test]
    fn persists_search_and_usage() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("glass.db");
        {
            let mut db = Database::open(&path).unwrap();
            db.replace_all(&[
                item("MainWindow.xaml", r"C:\Test\MainWindow.xaml"),
                item("Project notes", r"C:\Test\Project notes.txt"),
            ])
            .unwrap();
            db.record_open(r"C:\Test\Project notes.txt").unwrap();
            db.replace_all(&[
                item("MainWindow.xaml", r"C:\Test\MainWindow.xaml"),
                item("Project notes", r"C:\Test\Project notes.txt"),
            ])
            .unwrap();
            assert_eq!(
                db.search_candidates("mainwin", 10).unwrap()[0].title,
                "MainWindow.xaml"
            );
        }
        let db = Database::open(&path).unwrap();
        assert_eq!(db.count().unwrap(), 2);
        assert_eq!(
            db.search_candidates("", 10).unwrap()[0].title,
            "Project notes"
        );
    }

    #[test]
    fn upsert_rename_remove_and_deduplicate() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(&dir.path().join("glass.db")).unwrap();
        let original = item("Alpha", r"C:\Test\Alpha.txt");
        db.replace_all(&[original.clone(), original]).unwrap();
        assert_eq!(db.count().unwrap(), 1);
        db.remove_path(r"C:\Test\Alpha.txt").unwrap();
        db.upsert(&item("Beta", r"C:\Test\Beta.txt")).unwrap();
        assert!(!db.contains_path(r"C:\Test\Alpha.txt").unwrap());
        assert!(db.contains_path(r"C:\Test\Beta.txt").unwrap());
        db.set_favorite(r"C:\Test\Beta.txt", true).unwrap();
        assert!(db.search_candidates("", 10).unwrap()[0].favorite);
    }

    #[test]
    fn watcher_batch_removes_descendants_and_keeps_case_identity_unique() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(&dir.path().join("glass.db")).unwrap();
        db.replace_all(&[
            item("Folder", r"C:\Test\Folder"),
            item("Child", r"C:\Test\Folder\Child.txt"),
        ])
        .unwrap();
        db.apply_index_changes(&[], &[r"c:\test\folder".into()])
            .unwrap();
        assert_eq!(db.count().unwrap(), 0);

        db.apply_index_changes(&[item("Glass", r"C:\Apps\GLASS.exe")], &[])
            .unwrap();
        db.apply_index_changes(&[item("Glass", r"c:\apps\glass.exe")], &[])
            .unwrap();
        assert_eq!(db.count().unwrap(), 1);
    }

    #[test]
    fn fuzzy_lookup_survives_a_wrong_first_character_and_usage_affects_score() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(&dir.path().join("glass.db")).unwrap();
        db.replace_all(&[
            item("Visual Studio Code", r"C:\Apps\Code.exe"),
            item("Notebook Alpha", r"C:\Test\Alpha.txt"),
            item("Notebook Beta", r"C:\Test\Beta.txt"),
        ])
        .unwrap();
        assert!(db
            .fuzzy_candidates("xisual studio code", 10)
            .unwrap()
            .iter()
            .any(|item| item.title == "Visual Studio Code"));
        db.record_open(r"C:\Test\Beta.txt").unwrap();
        db.record_open(r"C:\Test\Beta.txt").unwrap();
        let results = db.search_candidates("notebook", 10).unwrap();
        let alpha = results
            .iter()
            .find(|item| item.title == "Notebook Alpha")
            .unwrap()
            .score;
        let beta = results
            .iter()
            .find(|item| item.title == "Notebook Beta")
            .unwrap()
            .score;
        assert!(beta > alpha);
    }

    #[test]
    fn repairs_missing_fts_rows_and_archives_an_invalid_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("glass.db");
        {
            let mut db = Database::open(&path).unwrap();
            db.replace_all(&[item("Repair Me", r"C:\Test\Repair.txt")])
                .unwrap();
            db.connection.execute("DELETE FROM files_fts", []).unwrap();
        }
        let db = Database::open(&path).unwrap();
        assert_eq!(db.search_candidates("repair", 10).unwrap().len(), 1);
        drop(db);

        std::fs::write(&path, b"not a sqlite database").unwrap();
        let recovered = Database::open(&path).unwrap();
        assert_eq!(recovered.count().unwrap(), 0);
        assert!(std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains("corrupt-")));
    }

    #[test]
    fn clipboard_history_is_bounded_and_clearable() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("glass.db")).unwrap();
        for id in 1..=4 {
            db.save_clipboard_item(
                &ClipboardItem {
                    id,
                    text: format!("metin {id}"),
                    copied_at: id,
                },
                3,
            )
            .unwrap();
        }
        let items = db.load_clipboard_history(100).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].text, "metin 4");
        db.delete_clipboard_item(items[0].id).unwrap();
        assert_eq!(db.load_clipboard_history(100).unwrap().len(), 2);
        db.clear_clipboard_history().unwrap();
        assert!(db.load_clipboard_history(100).unwrap().is_empty());
    }

    #[test]
    #[ignore]
    fn performance_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(&dir.path().join("glass.db")).unwrap();
        let items: Vec<_> = (0..10_000)
            .map(|index| {
                item(
                    &format!("Project Document {index}"),
                    &format!(r"C:\Bench\Document{index}.txt"),
                )
            })
            .collect();
        let rebuild_started = std::time::Instant::now();
        db.replace_all(&items).unwrap();
        let rebuild_ms = rebuild_started.elapsed().as_secs_f64() * 1_000.0;
        let reader = Database::open(&dir.path().join("glass.db")).unwrap();
        let mut samples = Vec::new();
        for _ in 0..50 {
            let started = std::time::Instant::now();
            let results = reader.search_candidates("document", 50).unwrap();
            assert!(!results.is_empty());
            samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
        samples.sort_by(f64::total_cmp);
        let average = samples.iter().sum::<f64>() / samples.len() as f64;
        let p95 = samples[(samples.len() as f64 * 0.95) as usize - 1];
        let updated: Vec<_> = (0..10_000)
            .map(|index| {
                item(
                    &format!("Updated Document {index}"),
                    &format!(r"C:\Bench\Document{index}.txt"),
                )
            })
            .collect();
        let writer = std::thread::spawn(move || {
            db.replace_all(&updated).unwrap();
        });
        let mut indexing_samples = Vec::new();
        while !writer.is_finished() {
            let started = std::time::Instant::now();
            let results = reader.search_candidates("document", 50).unwrap();
            assert!(!results.is_empty());
            indexing_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
        writer.join().unwrap();
        let indexing_max = indexing_samples.into_iter().fold(0.0, f64::max);
        println!("PERF items=10000 rebuild_ms={rebuild_ms:.2} warm_search_avg_ms={average:.3} warm_search_p95_ms={p95:.3} indexing_search_max_ms={indexing_max:.3}");
    }
}
