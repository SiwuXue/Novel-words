use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;

pub struct PresetCatalog {
    pub path: PathBuf,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetManifest {
    pub version: u32,
    pub books: Vec<CatalogBook>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogBook {
    pub key: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub word_count: u32,
    pub file: String,
    pub sources: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogWord {
    word: String,
    word_key: String,
    definition: String,
    phonetic: String,
    example_sentence: String,
    memory_tag: String,
    sources: Vec<String>,
}

impl PresetCatalog {
    fn archive(&self) -> Result<zip::ZipArchive<std::fs::File>, String> {
        let file =
            std::fs::File::open(&self.path).map_err(|e| format!("无法打开离线预设资源: {}", e))?;
        zip::ZipArchive::new(file).map_err(|e| format!("预设资源损坏: {}", e))
    }
    pub fn manifest(&self) -> Result<PresetManifest, String> {
        let mut archive = self.archive()?;
        let file = archive
            .by_name("manifest.json")
            .map_err(|e| e.to_string())?;
        let manifest: PresetManifest = serde_json::from_reader(file).map_err(|e| e.to_string())?;
        if manifest.version != 1 || manifest.books.len() != 40 {
            return Err("不支持的预设资源版本或目录不完整".into());
        }
        let mut keys = std::collections::BTreeSet::new();
        for book in &manifest.books {
            if !keys.insert(&book.key)
                || book.word_count == 0
                || book.file != format!("data/{}.jsonl", book.key)
            {
                return Err("预设资源目录无效".into());
            }
        }
        Ok(manifest)
    }
    pub fn register(&self, db: &Connection) -> Result<PresetManifest, String> {
        let manifest = self.manifest()?;
        for book in &manifest.books {
            let id: Option<i64> = db
                .query_row(
                    "SELECT id FROM vocab_book WHERE is_preset=1 AND preset_key=?1 LIMIT 1",
                    [&book.key],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some(id) = id {
                db.execute(
                    "UPDATE vocab_book SET name=?1,description=?2 WHERE id=?3",
                    params![book.name, book.description, id],
                )
                .map_err(|e| e.to_string())?;
            } else {
                db.execute("INSERT INTO vocab_book(name,description,is_preset,preset_key) VALUES (?1,?2,1,?3)",params![book.name,book.description,book.key]).map_err(|e|e.to_string())?;
            }
        }
        Ok(manifest)
    }
    pub fn ensure_loaded(
        &self,
        db: &mut Connection,
        key: &str,
        progress: Option<&dyn Fn(u32, u32)>,
    ) -> Result<i64, String> {
        let manifest = self.register(db)?;
        // Keep existing legacy references exact; resolve absent old keys to their new category.
        let canonical = manifest.books.iter().any(|b| b.key == key);
        if !canonical {
            let existing:Option<(i64,u32)>=db.query_row("SELECT b.id,COUNT(w.id) FROM vocab_book b JOIN vocab_word w ON w.vocab_book_id=b.id WHERE b.is_preset=1 AND b.preset_key=?1 GROUP BY b.id",[key],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
            if let Some((id, count)) = existing {
                if count > 0 {
                    if let Some(progress) = progress {
                        progress(count, count);
                    }
                    return Ok(id);
                }
            }
        }
        let book = manifest
            .books
            .iter()
            .find(|b| {
                b.key == key
                    || b.sources.iter().any(|s| s == key)
                    || key == "cet4" && b.key == "cet4-all"
            })
            .ok_or_else(|| format!("未找到预设词表: {}", key))?;
        let id: i64 = db
            .query_row(
                "SELECT id FROM vocab_book WHERE is_preset=1 AND preset_key=?1",
                [&book.key],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let revision = format!("{}:{}", manifest.version, book.word_count);
        let setting = format!("preset_seed:{}", book.key);
        let loaded: Option<String> = db
            .query_row(
                "SELECT value FROM app_settings WHERE key=?1",
                [&setting],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let count: u32 = db
            .query_row(
                "SELECT COUNT(*) FROM vocab_word WHERE vocab_book_id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if loaded.as_deref() == Some(&revision) && count == book.word_count {
            if let Some(progress) = progress {
                progress(count, count);
            }
            return Ok(id);
        }
        if let Some(progress) = progress {
            progress(0, book.word_count);
        }
        let mut archive = self.archive()?;
        let mut bytes = String::new();
        archive
            .by_name(&book.file)
            .map_err(|e| e.to_string())?
            .read_to_string(&mut bytes)
            .map_err(|e| e.to_string())?;
        let mut words = Vec::with_capacity(book.word_count as usize);
        let mut keys = std::collections::BTreeSet::new();
        for line in bytes.lines().filter(|line| !line.trim().is_empty()) {
            let word: CatalogWord =
                serde_json::from_str(line).map_err(|e| format!("预设词条解析失败: {}", e))?;
            if word.word_key.is_empty()
                || word.word_key != crate::user_vocab::word_key(&word.word)
                || !keys.insert(word.word_key.clone())
                || word.sources.is_empty()
                || word.sources.iter().any(|s| !book.sources.contains(s))
            {
                return Err("预设词条或来源无效".into());
            }
            words.push(word);
        }
        if words.len() != book.word_count as usize {
            return Err("预设词条数量不完整，请重新加载".into());
        }
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM vocab_word WHERE vocab_book_id=?1", [id])
            .map_err(|e| e.to_string())?;
        {
            let mut stmt=tx.prepare("INSERT INTO vocab_word(vocab_book_id,word,word_key,definition,phonetic,example_sentence,memory_tag,source_keys) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e|e.to_string())?;
            for (index, word) in words.iter().enumerate() {
                stmt.execute(params![
                    id,
                    word.word,
                    word.word_key,
                    word.definition,
                    word.phonetic,
                    word.example_sentence,
                    word.memory_tag,
                    serde_json::to_string(&word.sources).map_err(|e| e.to_string())?
                ])
                .map_err(|e| e.to_string())?;
                if index % 100 == 0 || index + 1 == words.len() {
                    if let Some(progress) = progress {
                        progress(index as u32 + 1, book.word_count);
                    }
                }
            }
        }
        tx.execute(
            "INSERT OR REPLACE INTO app_settings(key,value) VALUES (?1,?2)",
            params![setting, revision],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn incomplete_resource_is_not_cached_and_can_be_retried() {
        use std::io::{Read, Write};
        let dir = std::env::temp_dir().join(format!(
            "nw-catalog-retry-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let state = crate::db::init_db(&dir).unwrap();
        let mut db = state.db.lock().unwrap();
        let original =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/presets.zip");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&original).unwrap()).unwrap();
        let mut manifest = String::new();
        archive
            .by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        drop(archive);
        let path = dir.join("broken.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        writer
            .start_file("manifest.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(manifest.as_bytes()).unwrap();
        writer
            .start_file(
                "data/cet6-all.jsonl",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.finish().unwrap();
        let catalog = super::PresetCatalog { path: path.clone() };
        assert!(catalog.ensure_loaded(&mut db, "cet6-all", None).is_err());
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM vocab_word", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM app_settings WHERE key='preset_seed:cet6-all'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        std::fs::copy(original, path).unwrap();
        let id = catalog.ensure_loaded(&mut db, "cet6-all", None).unwrap();
        assert_eq!(crate::user_vocab::load_words(&db, id).unwrap().len(), 3992);
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn catalog_registers_forty_without_personal_words_and_loads_on_demand() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/presets.zip");
        let dir = std::env::temp_dir().join(format!(
            "nw-catalog-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let state = crate::db::init_db(&dir).unwrap();
        let mut db = state.db.lock().unwrap();
        let catalog = super::PresetCatalog { path };
        let manifest = catalog.register(&db).unwrap();
        assert_eq!(manifest.books.len(), 40);
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM vocab_word", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let progress = std::cell::RefCell::new(vec![]);
        let id = catalog
            .ensure_loaded(
                &mut db,
                "cet6-all",
                Some(&|current, total| progress.borrow_mut().push((current, total))),
            )
            .unwrap();
        assert_eq!(crate::user_vocab::load_words(&db, id).unwrap().len(), 3992);
        assert_eq!(
            catalog.ensure_loaded(&mut db, "cet6-all", None).unwrap(),
            id
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM user_vocab", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            manifest
                .books
                .iter()
                .find(|b| b.key == "toefl-all")
                .unwrap()
                .word_count,
            10367
        );
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
