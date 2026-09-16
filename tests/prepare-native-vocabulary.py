"""Create a legacy backup fixture for the isolated native vocabulary test."""
from pathlib import Path
import sqlite3

root = Path(__file__).resolve().parents[1]
target = root / "test-data" / "native-vocabulary" / "native-vocabulary-legacy.db"
target.parent.mkdir(parents=True, exist_ok=True)
if target.exists():
    raise SystemExit("Legacy test fixture already exists; reusing it is safe.")
source = (root / "src-tauri/src/db.rs").read_text(encoding="utf-8")
ddl = source.split('pub(crate) const CREATE_TABLES_SQL: &str = "', 1)[1].split('";', 1)[0]
with sqlite3.connect(target) as conn:
    conn.executescript(ddl)
    conn.execute("INSERT INTO vocab_book(name) VALUES ('Legacy Context')")
    conn.execute("INSERT INTO vocab_word(vocab_book_id,word,definition,proficiency) VALUES (1,'garden','old backup','familiar')")
print(target)
