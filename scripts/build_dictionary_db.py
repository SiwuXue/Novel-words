"""
Build a read-only SQLite dictionary.db from DictionaryData CSVs.

Inputs:
  DictionaryData/word.csv            — 单词 + 音标 + 词频 (分隔符 '>')
  DictionaryData/word_translation.csv — 单词 + 中文翻译 (标准 CSV)

Output:
  src-tauri/resources/dictionary.db

Schema:
  CREATE TABLE dict_word (
      word         TEXT PRIMARY KEY,   -- 原始词形（保留大小写）
      phonetic_uk  TEXT,
      phonetic_us  TEXT,
      frequency    REAL,
      difficulty   INTEGER,
      translation  TEXT
  );
  CREATE INDEX idx_dict_word_translation ON dict_word(translation);

Query 通过 word = ? COLLATE NOCASE 实现大小写不敏感查询。
中文反向查询走 LIKE '%keyword%' 匹配 translation 字段。
"""

import csv
import os
import sqlite3
import sys

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(BASE_DIR)
DICT_DATA_DIR = os.path.join(PROJECT_ROOT, "DictionaryData")
WORD_CSV = os.path.join(DICT_DATA_DIR, "word.csv")
TRANS_CSV = os.path.join(DICT_DATA_DIR, "word_translation.csv")
OUT_DB = os.path.join(PROJECT_ROOT, "src-tauri", "resources", "dictionary.db")


def main():
    if not os.path.exists(WORD_CSV):
        print(f"[ERROR] 找不到 {WORD_CSV}", file=sys.stderr)
        sys.exit(1)
    if not os.path.exists(TRANS_CSV):
        print(f"[ERROR] 找不到 {TRANS_CSV}", file=sys.stderr)
        sys.exit(1)

    os.makedirs(os.path.dirname(OUT_DB), exist_ok=True)
    if os.path.exists(OUT_DB):
        os.remove(OUT_DB)

    conn = sqlite3.connect(OUT_DB)
    cur = conn.cursor()

    # 建表
    cur.execute("""
        CREATE TABLE dict_word (
            word         TEXT PRIMARY KEY,
            phonetic_uk  TEXT,
            phonetic_us  TEXT,
            frequency    REAL,
            difficulty   INTEGER,
            translation  TEXT
        )
    """)
    cur.execute("CREATE INDEX idx_dict_word_translation ON dict_word(translation)")

    # Step A: 读取 word.csv -> { vocabulary: (phonetic_uk, phonetic_us, frequency, difficulty) }
    word_meta = {}
    with open(WORD_CSV, "r", encoding="utf-8") as f:
        reader = csv.reader(f, delimiter=">")
        header = next(reader, None)
        # 列顺序: vc_id, vc_vocabulary, vc_phonetic_uk, vc_phonetic_us,
        #         vc_frequency, vc_difficulty, vc_acknowledge_rate
        for row in reader:
            if len(row) < 7:
                continue
            vocab = row[1].strip()
            if not vocab:
                continue
            try:
                freq = float(row[4]) if row[4] else 0.0
            except ValueError:
                freq = 0.0
            try:
                diff = int(row[5]) if row[5] else 0
            except ValueError:
                diff = 0
            word_meta[vocab] = (row[2].strip(), row[3].strip(), freq, diff)
    print(f"[INFO] word.csv 加载 {len(word_meta)} 条元数据")

    # Step B: 读取 word_translation.csv 并合并写入
    batch = []
    batch_size = 5000
    total = 0
    only_in_trans = 0

    with open(TRANS_CSV, "r", encoding="utf-8") as f:
        reader = csv.reader(f)
        header = next(reader, None)
        # 列顺序: word, translation
        for row in reader:
            if len(row) < 2:
                continue
            word = row[0].strip()
            translation = row[1].strip()
            if not word:
                continue
            meta = word_meta.get(word)
            if meta is None:
                # translation 表里有但 word.csv 没有，保留释义，音标为空
                only_in_trans += 1
                phonetic_uk, phonetic_us, freq, diff = "", "", 0.0, 0
            else:
                phonetic_uk, phonetic_us, freq, diff = meta
            batch.append((word, phonetic_uk, phonetic_us, freq, diff, translation))
            if len(batch) >= batch_size:
                cur.executemany(
                    "INSERT OR REPLACE INTO dict_word "
                    "(word, phonetic_uk, phonetic_us, frequency, difficulty, translation) "
                    "VALUES (?, ?, ?, ?, ?, ?)",
                    batch,
                )
                total += len(batch)
                conn.commit()
                batch.clear()
                print(f"[INFO] 已写入 {total} 条...", end="\r")
    # flush remainder
    if batch:
        cur.executemany(
            "INSERT OR REPLACE INTO dict_word "
            "(word, phonetic_uk, phonetic_us, frequency, difficulty, translation) "
            "VALUES (?, ?, ?, ?, ?, ?)",
            batch,
        )
        total += len(batch)
        conn.commit()
    print(f"\n[INFO] translation 写入 {total} 条（其中 {only_in_trans} 条无音标）")

    # Step C: 补充 word.csv 中未出现在 translation 的单词（无释义）
    only_in_word = 0
    cur.execute("SELECT word FROM dict_word")
    existing = {row[0] for row in cur.fetchall()}
    extra = []
    for vocab, (uk, us, freq, diff) in word_meta.items():
        if vocab not in existing:
            extra.append((vocab, uk, us, freq, diff, ""))
            only_in_word += 1
    if extra:
        for i in range(0, len(extra), batch_size):
            cur.executemany(
                "INSERT OR REPLACE INTO dict_word "
                "(word, phonetic_uk, phonetic_us, frequency, difficulty, translation) "
                "VALUES (?, ?, ?, ?, ?, ?)",
                extra[i : i + batch_size],
            )
        conn.commit()
    print(f"[INFO] word.csv-only 补写 {only_in_word} 条（无释义）")

    # Step D: VACUUM 压缩
    cur.execute("VACUUM")
    conn.close()

    size_mb = os.path.getsize(OUT_DB) / (1024 * 1024)
    print(f"[OK] 生成 {OUT_DB}")
    print(f"[OK] 文件大小: {size_mb:.2f} MB")
    print(f"[OK] 总词条数: {total + only_in_word}")


if __name__ == "__main__":
    main()
