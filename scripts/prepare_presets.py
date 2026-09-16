"""Build the 40 offline presets from the 67 supplied book ZIPs (Python 3.8+).

No packages or machine-specific paths are required. Run with --help for paths.
The source ZIPs hold JSON Lines despite their .json extension. No extraction,
network access, stemming, punctuation removal, or source rank cutoff is used.
"""

import argparse
from collections import defaultdict
import json
from pathlib import Path
import sys
import zipfile


PROJECT_ROOT = Path(__file__).resolve().parent.parent
ZIP_TIMESTAMP = (1980, 1, 1, 0, 0, 0)

# Explicit source membership makes missing editions a build error. "luan"
# identifies a shuffled edition, not a separate canonical examination book.
EXAMS = (
    ("cet4-all", "大学英语四级", "university", "CET4", ("_1", "_2", "_3", "luan_1", "luan_2")),
    ("cet6-all", "大学英语六级", "university", "CET6", ("_1", "_2", "_3", "luan_1")),
    ("kaoyan-all", "考研英语", "university", "KaoYan", ("_1", "_2", "_3", "luan_1")),
    ("tem4-all", "英语专业四级", "university", "Level4", ("_1", "_2", "luan_1", "luan_2")),
    ("tem8-all", "英语专业八级", "university", "Level8", ("_1", "_2", "luan_2")),
    ("ielts-all", "雅思 IELTS", "international", "IELTS", ("_2", "_3", "luan_2")),
    ("toefl-all", "托福 TOEFL", "international", "TOEFL", ("_2", "_3")),
    ("gre-all", "GRE", "international", "GRE", ("_2", "_3")),
    ("gmat-all", "GMAT", "international", "GMAT", ("_2", "_3", "luan_2")),
    ("sat-all", "SAT", "international", "SAT", ("_2", "_3")),
    ("bec-all", "商务英语 BEC", "international", "BEC", ("_2", "_3")),
    ("junior-exam-all", "中考英语", "school", "ChuZhong", ("_2", "_3", "luan_2")),
    ("senior-exam-all", "高考英语", "school", "GaoZhong", ("_2", "_3", "luan_2")),
)
CHINESE_GRADES = {3: "三", 4: "四", 5: "五", 6: "六", 7: "七", 8: "八", 9: "九"}


def book_specs():
    """Return canonical books in display order, with exact expected sources."""
    books = []
    for key, name, category, prefix, suffixes in EXAMS:
        sources = sorted(prefix + suffix for suffix in suffixes)
        books.append({"key": key, "name": name, "category": category, "sources": sources,
                      "description": "合并所提供的%d个来源版本，按规范化词形去重，保留不同词性、释义与来源。" % len(sources)})
    for prefix, name, grade_terms in (
        ("PEPXiaoXue", "人教版小学英语", [(grade, (1, 2)) for grade in range(3, 7)]),
        ("PEPChuZhong", "人教版初中英语", [(7, (1, 2)), (8, (1, 2)), (9, (1,))]),
    ):
        for grade, terms in grade_terms:
            for term in terms:
                key = "%s%d_%d" % (prefix, grade, term)
                title = "%s%s年级%s册" % (name, CHINESE_GRADES[grade], "上" if term == 1 else "下")
                books.append({"key": key, "name": title, "category": "textbook", "sources": [key],
                              "description": "所提供的教材单册词汇，按规范化词形去重；来源：%s。" % key})
    for prefix, name, volumes in (
        ("PEPGaoZhong", "人教版高中英语", range(1, 12)),
        ("BeiShiGaoZhong", "北师大版高中英语", (2, 3, 7)),
    ):
        for volume in volumes:
            key = "%s_%d" % (prefix, volume)
            description = "所提供的教材单册词汇，按规范化词形去重；来源：%s。" % key
            if prefix == "BeiShiGaoZhong":
                description += "当前资源仅提供第2、3、7册。"
            books.append({"key": key, "name": "%s第%d册" % (name, volume), "category": "textbook",
                          "sources": [key], "description": description})
    return books


def clean_text(value):
    if value is None:
        return ""
    if not isinstance(value, str):
        raise ValueError("expected text, received %r" % type(value).__name__)
    return " ".join(value.split())


def normalize_word_key(word):
    return clean_text(word).lower().replace("’", "'").replace("‘", "'")


def json_text(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def discover_sources(source_dir, expected):
    sources = {}
    for path in sorted(Path(source_dir).glob("*.zip")):
        with zipfile.ZipFile(path) as archive:
            members = [info.filename for info in archive.infolist()
                       if not info.is_dir() and info.filename.lower().endswith(".json")]
        if len(members) != 1:
            raise ValueError("%s: expected one JSON Lines member, found %d" % (path.name, len(members)))
        member = members[0]
        key = Path(member).stem
        if key not in expected:
            raise ValueError("unknown source: %s (%s)" % (key, path.name))
        if key in sources:
            raise ValueError("duplicate source: %s" % key)
        sources[key] = (path, member)
    missing = sorted(expected - set(sources))
    if missing:
        raise ValueError("missing source archives: " + ", ".join(missing))
    return sources


def read_source(path, member, source_key):
    """Validate every row and retain only fields belonging to this headword."""
    count = 0
    with zipfile.ZipFile(path) as archive:
        text = archive.read(member).decode("utf-8-sig")
    for line_number, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
            if not isinstance(entry, dict) or entry.get("bookId") != source_key:
                raise ValueError("entry bookId does not match the archive source")
            word = clean_text(entry.get("headWord"))
            word_key = normalize_word_key(word)
            if not word_key:
                raise ValueError("blank headWord")
            content = entry["content"]["word"]["content"]
            senses = []
            for trans in content.get("trans") or []:
                meaning = clean_text(trans.get("tranCn"))
                if meaning:
                    # Keep original POS labels, only avoid adding a second dot.
                    senses.append((clean_text(trans.get("pos")).rstrip("."), meaning))
            sentences = []
            for sentence in (content.get("sentence") or {}).get("sentences") or []:
                en, cn = clean_text(sentence.get("sContent")), clean_text(sentence.get("sCn"))
                if en or cn:
                    sentences.append((en, cn))
            phrases = []
            for phrase in (content.get("phrase") or {}).get("phrases") or []:
                en, cn = clean_text(phrase.get("pContent")), clean_text(phrase.get("pCn"))
                if en or cn:
                    phrases.append((en, cn))
            memory = content.get("remMethod") or {}
            record = {"word": word, "source": source_key, "senses": sorted(set(senses)),
                      "sentences": sorted(set(sentences)), "phrases": sorted(set(phrases)),
                      "usphone": clean_text(content.get("usphone")), "ukphone": clean_text(content.get("ukphone")),
                      "phone": clean_text(content.get("phone")),
                      "memory": clean_text(memory.get("val")), "memoryDesc": clean_text(memory.get("desc"))}
        except (ValueError, KeyError, TypeError, AttributeError) as error:
            raise ValueError("%s:%d: %s" % (source_key, line_number, error)) from error
        count += 1
        yield word_key, record
    if not count:
        raise ValueError("%s: source contains no words" % source_key)


def completeness(record):
    """Higher scores select richer source records; final ties are lexical."""
    return (len(record["senses"]), sum(len(meaning) for _, meaning in record["senses"]),
            any(en and cn for en, cn in record["sentences"]),
            bool(record["usphone"] or record["ukphone"] or record["phone"]),
            len(record["memory"]), len(record["phrases"]),
            sum(len(en) + len(cn) for en, cn in record["sentences"]))


def merge_word(word_key, records):
    records = sorted(records, key=lambda record: (tuple(-int(value) for value in completeness(record)),
                                                 record["source"], json_text(record)))
    senses = sorted({sense for record in records for sense in record["senses"]})
    definition = [((pos + ". ") if pos else "") + meaning for pos, meaning in senses]
    if not definition:
        definition.append("（暂无释义）")
    memory_candidates = [record for record in records if record["memory"]]
    memory = min(memory_candidates, key=lambda record: (-len(record["memory"]),
                                                        tuple(-int(value) for value in completeness(record)),
                                                        record["source"], json_text(record))) if memory_candidates else None
    if memory:
        definition.append("【记忆】" + memory["memory"])
    phrases = sorted({phrase for record in records for phrase in record["phrases"]},
                     key=lambda phrase: (normalize_word_key(phrase[0]), phrase[1], phrase[0]))
    # Collapse typographic/case differences in English while retaining distinct
    # Chinese translations of the same phrase. The lexical choice is stable.
    unique_phrases, phrase_keys = [], set()
    for en, cn in phrases:
        key = (normalize_word_key(en), cn)
        if key not in phrase_keys:
            phrase_keys.add(key)
            unique_phrases.append((en, cn))
    if unique_phrases:
        definition.append("【搭配】")
        definition.extend("  · " + " ".join(part for part in phrase if part) for phrase in unique_phrases[:5])
    examples = [(en, cn, record) for record in records for en, cn in record["sentences"]]
    best_example = min(examples, key=lambda item: (-int(bool(item[0] and item[1])),
                                                 tuple(-int(value) for value in completeness(item[2])),
                                                 -len(item[0]) - len(item[1]), item[2]["source"], item[0], item[1])) if examples else None
    # Preserve the application's US -> UK -> generic pronunciation preference,
    # then use the most complete record offering that pronunciation field.
    phonetic = next((record[field] for field in ("usphone", "ukphone", "phone")
                     for record in records if record[field]), "")
    return {"word": records[0]["word"], "wordKey": word_key,
            "definition": "\n".join(definition), "phonetic": phonetic,
            "exampleSentence": "\n".join(part for part in best_example[:2] if part) if best_example else "",
            "memoryTag": (memory["memoryDesc"] or memory["memory"]) if memory else "",
            "sources": sorted({record["source"] for record in records})}


def build_archive(source_dir, output_path):
    """Build a fully validated archive; return the manifest used by the app."""
    specs = book_specs()
    expected = {source for book in specs for source in book["sources"]}
    sources = discover_sources(source_dir, expected)
    files, books = {}, []
    for spec in specs:
        words = defaultdict(list)
        for source_key in spec["sources"]:
            path, member = sources[source_key]
            for word_key, record in read_source(path, member, source_key):
                words[word_key].append(record)
        rows = [merge_word(key, words[key]) for key in sorted(words)]
        file = "data/%s.jsonl" % spec["key"]
        files[file] = ("\n".join(json_text(row) for row in rows) + "\n").encode("utf-8")
        books.append(dict(spec, wordCount=len(rows), file=file))
    manifest = {"version": 1, "books": books}
    files["manifest.json"] = (json_text(manifest) + "\n").encode("utf-8")
    output_path = Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name in sorted(files):
            info = zipfile.ZipInfo(name, date_time=ZIP_TIMESTAMP)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, files[name], compresslevel=9)
    return manifest


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=PROJECT_ROOT / "DictionaryData" / "book",
                        help="directory containing all 67 source ZIPs (default: DictionaryData/book)")
    parser.add_argument("--output", type=Path, default=PROJECT_ROOT / "src-tauri" / "resources" / "presets.zip",
                        help="output preset ZIP (default: src-tauri/resources/presets.zip)")
    args = parser.parse_args(argv)
    try:
        manifest = build_archive(args.source, args.output)
    except (ValueError, OSError, zipfile.BadZipFile, UnicodeError) as error:
        parser.exit(1, "preset build failed: %s\n" % error)
    print("Built %d books from %d sources; %d deduplicated book memberships; %d bytes: %s" %
          (len(manifest["books"]), sum(len(book["sources"]) for book in manifest["books"]),
           sum(book["wordCount"] for book in manifest["books"]), args.output.stat().st_size, args.output))
    return 0


if __name__ == "__main__":
    sys.exit(main())
