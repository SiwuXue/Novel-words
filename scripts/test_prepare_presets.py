"""Real ZIP fixtures for the dependency-free offline preset conversion."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile


MODULE_PATH = Path(__file__).with_name("prepare_presets.py")
prepare = None
if MODULE_PATH.exists():
    spec = importlib.util.spec_from_file_location("prepare_presets", MODULE_PATH)
    prepare = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(prepare)

EXAM_SOURCES = {
    "CET4": ("_1", "_2", "_3", "luan_1", "luan_2"),
    "CET6": ("_1", "_2", "_3", "luan_1"),
    "KaoYan": ("_1", "_2", "_3", "luan_1"),
    "Level4": ("_1", "_2", "luan_1", "luan_2"),
    "Level8": ("_1", "_2", "luan_2"),
    "IELTS": ("_2", "_3", "luan_2"),
    "TOEFL": ("_2", "_3"),
    "GRE": ("_2", "_3"),
    "GMAT": ("_2", "_3", "luan_2"),
    "SAT": ("_2", "_3"),
    "BEC": ("_2", "_3"),
    "ChuZhong": ("_2", "_3", "luan_2"),
    "GaoZhong": ("_2", "_3", "luan_2"),
}
SOURCE_KEYS = sorted(
    [prefix + suffix for prefix, suffixes in EXAM_SOURCES.items() for suffix in suffixes]
    + ["PEPXiaoXue%d_%d" % (grade, term) for grade in range(3, 7) for term in (1, 2)]
    + ["PEPChuZhong%d_%d" % (grade, term) for grade, terms in ((7, (1, 2)), (8, (1, 2)), (9, (1,))) for term in terms]
    + ["PEPGaoZhong_%d" % volume for volume in range(1, 12)]
    + ["BeiShiGaoZhong_%d" % volume for volume in (2, 3, 7)]
)


def entry(source, word="shared", definitions=None, **content):
    content["trans"] = definitions if definitions is not None else [{"pos": "n", "tranCn": "共用"}]
    return {"headWord": word, "bookId": source, "content": {"word": {"content": content}}}


class PresetConversionTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(prepare, "offline preset converter has not been implemented")
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "source"
        self.source.mkdir()
        for index, key in enumerate(SOURCE_KEYS):
            self.write_source(key, [entry(key)], index)

    def write_source(self, key, rows, index=0, timestamp=(2010, 1, 1, 0, 0, 0)):
        paths = list(self.source.glob("*_%s.zip" % key))
        path = paths[0] if paths else self.source / ("%d_%s.zip" % (index, key))
        with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            info = zipfile.ZipInfo(key + ".json", date_time=timestamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, "\n".join(json.dumps(row, ensure_ascii=False) for row in rows))
        return path

    def build(self, filename="presets.zip"):
        path = self.root / filename
        manifest = prepare.build_archive(self.source, path)
        return path, manifest

    def words(self, path, key):
        with zipfile.ZipFile(path) as archive:
            return [json.loads(line) for line in archive.read("data/%s.jsonl" % key).decode("utf-8").splitlines()]

    def test_normalization_preserves_punctuation_and_inflections(self):
        self.assertEqual(prepare.normalize_word_key("  Don’t\t\n give ‘up’  "), "don't give 'up'")
        self.assertEqual(prepare.normalize_word_key("Cats!"), "cats!")
        self.assertNotEqual(prepare.normalize_word_key("cats"), prepare.normalize_word_key("cat"))

    def test_manifest_contains_13_merged_exams_and_27_individual_textbooks(self):
        path, manifest = self.build()
        self.assertEqual(manifest["version"], 1)
        self.assertEqual(len(manifest["books"]), 40)
        self.assertEqual(sum(book["category"] == "textbook" for book in manifest["books"]), 27)
        self.assertEqual(sorted(source for book in manifest["books"] for source in book["sources"]), SOURCE_KEYS)
        books = {book["key"]: book for book in manifest["books"]}
        self.assertEqual(books["cet6-all"]["sources"], ["CET6_1", "CET6_2", "CET6_3", "CET6luan_1"])
        self.assertEqual(books["toefl-all"]["category"], "international")
        self.assertEqual(books["junior-exam-all"]["category"], "school")
        self.assertIn("三年级上册", books["PEPXiaoXue3_1"]["name"])
        self.assertIn("第11册", books["PEPGaoZhong_11"]["name"])
        self.assertEqual(sorted(key for key in books if key.startswith("BeiShi")), ["BeiShiGaoZhong_2", "BeiShiGaoZhong_3", "BeiShiGaoZhong_7"])
        with zipfile.ZipFile(path) as archive:
            self.assertEqual(len(archive.namelist()), 41)
            self.assertEqual(json.loads(archive.read("manifest.json")), manifest)
        for book in manifest["books"]:
            self.assertEqual(book["wordCount"], len(self.words(path, book["key"])))
            self.assertEqual(set(book), {"key", "name", "description", "category", "wordCount", "file", "sources"})

    def test_word_deduplication_keeps_all_distinct_senses_and_sources(self):
        self.write_source("CET6_1", [entry("CET6_1", "  Look\tUP ", [{"pos": "v", "tranCn": " 查找 "}]), entry("CET6_1", "look up", [{"pos": "v", "tranCn": "查找"}])])
        self.write_source("CET6_2", [entry("CET6_2", "LOOK UP", [{"pos": "n.", "tranCn": "检索"}, {"pos": "v", "tranCn": "抬头"}])])
        path, manifest = self.build()
        row = next(row for row in self.words(path, "cet6-all") if row["wordKey"] == "look up")
        self.assertEqual(row["definition"], "n. 检索\nv. 抬头\nv. 查找")
        self.assertEqual(row["sources"], ["CET6_1", "CET6_2"])
        self.assertEqual(next(book["wordCount"] for book in manifest["books"] if book["key"] == "cet6-all"), 2)
        self.assertEqual(set(row), {"word", "wordKey", "definition", "phonetic", "exampleSentence", "memoryTag", "sources"})

    def test_examples_prefer_bilingual_and_memory_uses_richer_method(self):
        self.write_source("CET6_1", [entry("CET6_1", "shared", usphone="abc", sentence={"sentences": [{"sContent": "A long English sentence without a translation."}]}, remMethod={"val": "短", "desc": "记忆"})])
        self.write_source("CET6_2", [entry("CET6_2", "shared", ukphone="xyz", sentence={"sentences": [{"sContent": "English only."}, {"sContent": "Share a book.", "sCn": "分享一本书。"}]}, remMethod={"val": "较完整的联想记忆方法", "desc": "联想"})])
        path, _ = self.build()
        row = self.words(path, "cet6-all")[0]
        self.assertEqual(row["exampleSentence"], "Share a book.\n分享一本书。")
        self.assertIn("【记忆】较完整的联想记忆方法", row["definition"])
        self.assertEqual(row["memoryTag"], "联想")
        self.assertEqual(row["phonetic"], "abc")

    def test_phrases_are_deduplicated_and_capped_at_five(self):
        phrases = [{"pContent": "phrase %d" % index, "pCn": "搭配%d" % index} for index in range(7)]
        self.write_source("CET6_1", [entry("CET6_1", phrase={"phrases": phrases + [phrases[0], {"pContent": "", "pCn": ""}]})])
        self.write_source("CET6_2", [entry("CET6_2", phrase={"phrases": list(reversed(phrases))})])
        path, _ = self.build()
        definition = self.words(path, "cet6-all")[0]["definition"]
        self.assertEqual(definition, "n. 共用\n【搭配】\n  · phrase 0 搭配0\n  · phrase 1 搭配1\n  · phrase 2 搭配2\n  · phrase 3 搭配3\n  · phrase 4 搭配4")

    def test_missing_source_is_rejected_before_output_is_written(self):
        next(self.source.glob("*_TOEFL_3.zip")).unlink()
        with self.assertRaisesRegex(ValueError, "TOEFL_3"):
            self.build()
        self.assertFalse((self.root / "presets.zip").exists())

    def test_unknown_source_is_rejected(self):
        self.write_source("Unknown_1", [entry("Unknown_1")])
        with self.assertRaisesRegex(ValueError, "Unknown_1"):
            self.build()

    def test_mismatched_source_id_is_rejected(self):
        self.write_source("CET6_1", [entry("CET4_1")])
        with self.assertRaisesRegex(ValueError, "CET6_1"):
            self.build()

    def test_blank_word_is_rejected_instead_of_silently_losing_a_row(self):
        self.write_source("CET6_1", [entry("CET6_1", "  ")])
        with self.assertRaisesRegex(ValueError, "CET6_1"):
            self.build()

    def test_archive_is_repeatable_when_source_order_and_timestamps_change(self):
        rows = [entry("CET6_1", "zebra"), entry("CET6_1", "Alpha")]
        self.write_source("CET6_1", rows)
        first, _ = self.build("first.zip")
        self.write_source("CET6_1", list(reversed(rows)), timestamp=(2030, 12, 31, 23, 59, 58))
        second, _ = self.build("second.zip")
        self.assertEqual(first.read_bytes(), second.read_bytes())
        with zipfile.ZipFile(first) as archive:
            self.assertTrue(all(info.date_time == (1980, 1, 1, 0, 0, 0) for info in archive.infolist()))
            self.assertEqual(archive.namelist(), sorted(archive.namelist()))
        self.assertEqual([row["wordKey"] for row in self.words(first, "cet6-all")], ["alpha", "shared", "zebra"])


class BundledPresetContractTests(unittest.TestCase):
    """Audit the actual bundled artifact, not just the miniature fixtures."""

    def test_bundled_resource_covers_all_source_books_with_real_counts(self):
        path = Path(__file__).resolve().parent.parent / "src-tauri" / "resources" / "presets.zip"
        self.assertTrue(path.exists(), "generate the bundled presets.zip before running its audit")
        distinct, total = set(), 0
        with zipfile.ZipFile(path) as archive:
            manifest = json.loads(archive.read("manifest.json"))
            self.assertEqual(manifest["version"], 1)
            self.assertEqual(len(manifest["books"]), 40)
            self.assertEqual(sorted(source for book in manifest["books"] for source in book["sources"]), SOURCE_KEYS)
            self.assertEqual(archive.namelist(), sorted(archive.namelist()))
            self.assertEqual(set(archive.namelist()), {"manifest.json"} | {book["file"] for book in manifest["books"]})
            self.assertTrue(all(info.date_time == (1980, 1, 1, 0, 0, 0) for info in archive.infolist()))
            for book in manifest["books"]:
                rows = [json.loads(line) for line in archive.read(book["file"]).decode("utf-8").splitlines()]
                keys = [row["wordKey"] for row in rows]
                self.assertEqual(book["wordCount"], len(rows), book["key"])
                self.assertEqual(keys, sorted(set(keys)), book["key"])
                cited_sources = set()
                for row in rows:
                    self.assertEqual(set(row), {"word", "wordKey", "definition", "phonetic", "exampleSentence", "memoryTag", "sources"})
                    self.assertTrue(all(isinstance(row[field], str) for field in ("word", "wordKey", "definition", "phonetic", "exampleSentence", "memoryTag")))
                    self.assertEqual(row["wordKey"], prepare.normalize_word_key(row["word"]))
                    self.assertTrue(row["wordKey"] and row["definition"])
                    self.assertEqual(row["sources"], sorted(set(row["sources"])))
                    self.assertTrue(row["sources"])
                    self.assertTrue(set(row["sources"]) <= set(book["sources"]))
                    cited_sources.update(row["sources"])
                self.assertEqual(cited_sources, set(book["sources"]), book["key"])
                distinct.update(keys)
                total += len(rows)
            books = {book["key"]: book for book in manifest["books"]}
            self.assertEqual(books["cet6-all"]["wordCount"], 3992)
            self.assertEqual(books["toefl-all"]["wordCount"], 10367)
            self.assertIsNone(archive.testzip())
        self.assertEqual(total, 80206)
        self.assertEqual(len(distinct), 23302)


if __name__ == "__main__":
    unittest.main()
