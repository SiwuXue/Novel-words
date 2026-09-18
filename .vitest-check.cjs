"use strict";
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);
var ttsPlayer_exports = {};
__export(ttsPlayer_exports, {
  splitSentenceSpans: () => splitSentenceSpans,
  splitSentences: () => splitSentences,
  ttsPlayer: () => ttsPlayer
});
module.exports = __toCommonJS(ttsPlayer_exports);
var import_core = require("@tauri-apps/api/core");
var import_vue = require("vue");
function splitSentences(text, maxLen = 300) {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (!normalized) return [];
  const pieces = normalized.split(/(?<=[。！？!?；;…])\s*|(?<=[.!?])\s+(?=["“''(A-Z])/).map((s) => s.trim()).filter(Boolean);
  const out = [];
  let buffer = "";
  for (const piece of pieces) {
    if ((buffer + piece).length > maxLen && buffer) {
      out.push(buffer);
      buffer = "";
    }
    if (piece.length > maxLen) {
      for (let i = 0; i < piece.length; i += maxLen) {
        out.push(piece.slice(i, i + maxLen));
      }
      continue;
    }
    buffer = buffer ? `${buffer} ${piece}` : piece;
  }
  if (buffer) out.push(buffer);
  return out;
}
function splitSentenceSpans(text, maxLen = 300) {
  const boundary = /(?<=[。！？!?；;…])\s*|(?<=[.!?])\s+(?=["“''(A-Z])/g;
  const raw = [];
  let match;
  let last = 0;
  while ((match = boundary.exec(text)) !== null) {
    if (match.index > last) {
      raw.push({ start: last, end: match.index });
      last = match.index + match[0].length;
    }
    if (match[0] === "") boundary.lastIndex++;
  }
  if (last < text.length) raw.push({ start: last, end: text.length });
  const spans = [];
  let buf = null;
  for (const seg of raw) {
    let { start, end } = seg;
    while (start < end && /\s/.test(text[start])) start++;
    while (end > start && /\s/.test(text[end - 1])) end--;
    const piece = text.slice(start, end);
    if (!piece) continue;
    if (piece.length > maxLen) {
      if (buf) {
        spans.push(buf);
        buf = null;
      }
      for (let i = start; i < end; i += maxLen) {
        spans.push({ text: text.slice(i, i + maxLen), start: i, end: Math.min(i + maxLen, end) });
      }
      continue;
    }
    if (!buf) {
      buf = { text: piece, start, end };
    } else if (end - buf.start <= maxLen) {
      buf.text = text.slice(buf.start, end);
      buf.end = end;
    } else {
      spans.push(buf);
      buf = { text: piece, start, end };
    }
  }
  if (buf) spans.push(buf);
  return spans;
}
function bytesToDataUrl(bytes) {
  let binary = "";
  for (let i = 0; i < bytes.length; i += 32768) {
    binary += String.fromCharCode(...bytes.slice(i, i + 32768));
  }
  return `data:audio/mpeg;base64,${btoa(binary)}`;
}
class TtsPlayer {
  /** 播放令牌：start/stop 递增；旧队列检测到令牌变化即中断 */
  token = 0;
  paused = false;
  audio = null;
  currentUtterance = null;
  /** idle / playing / paused（响应式） */
  _state = (0, import_vue.ref)("idle");
  get state() {
    return this._state.value;
  }
  currentIndex = (0, import_vue.ref)(-1);
  totalSentences = (0, import_vue.ref)(0);
  async start(sentences, settings, handlers = {}) {
    this.stop(true);
    const myToken = ++this.token;
    if (sentences.length === 0) {
      handlers.onFinish?.(false);
      return;
    }
    this.paused = false;
    this._state.value = "playing";
    this.totalSentences.value = sentences.length;
    this.currentIndex.value = -1;
    for (let i = 0; i < sentences.length; i++) {
      if (myToken !== this.token) return;
      while (this.paused && myToken === this.token) {
        await this.sleep(120);
        if (myToken !== this.token) return;
      }
      this.currentIndex.value = i;
      handlers.onSentenceStart?.(i, sentences.length, sentences[i]);
      try {
        await this.speakOne(sentences[i], settings, myToken);
      } catch (e) {
        if (myToken !== this.token) return;
        console.error("[ttsPlayer] sentence failed:", e);
        if (settings.provider === "edge") {
          try {
            await this.speakSystem(sentences[i], settings);
          } catch {
          }
        }
      }
    }
    if (myToken === this.token) {
      this._state.value = "idle";
      this.currentIndex.value = -1;
      handlers.onFinish?.(true);
    }
  }
  /** 单句合成并等待播放结束 */
  speakOne(text, settings, myToken) {
    if (settings.provider === "edge") {
      return this.speakEdge(text, settings, myToken);
    }
    return this.speakSystem(text, settings);
  }
  async speakEdge(text, settings, myToken) {
    const rate = Math.round((settings.rate - 1) * 100);
    const pitch = Math.round((settings.pitch - 1) * 100);
    const volume = settings.volume - 100;
    const bytes = await (0, import_core.invoke)("tts_synthesize", {
      text,
      voice: settings.voice,
      rate,
      pitch,
      volume
    });
    if (myToken !== this.token) return;
    const url = bytesToDataUrl(bytes);
    await this.playAudioUrl(url, myToken, settings.volume / 100);
  }
  playAudioUrl(url, myToken, volumeScale) {
    return new Promise((resolve, reject) => {
      if (myToken !== this.token) return resolve();
      const audio = new Audio(url);
      audio.volume = Math.min(1, Math.max(0, volumeScale));
      this.audio = audio;
      audio.onended = () => {
        if (this.audio === audio) this.audio = null;
        resolve();
      };
      audio.onerror = () => {
        if (this.audio === audio) this.audio = null;
        reject(new Error("audio playback failed"));
      };
      void audio.play().then(() => {
        if (this.paused && this.audio === audio) audio.pause();
      }).catch((e) => {
        if (this.audio === audio) this.audio = null;
        reject(e);
      });
    });
  }
  speakSystem(text, settings) {
    return new Promise((resolve, reject) => {
      if (!("speechSynthesis" in window)) {
        reject(new Error("\u5F53\u524D\u73AF\u5883\u4E0D\u652F\u6301\u7CFB\u7EDF\u8BED\u97F3"));
        return;
      }
      const utter = new SpeechSynthesisUtterance(text);
      utter.voice = window.speechSynthesis.getVoices().find((v) => v.name === settings.voice) ?? null;
      utter.lang = utter.voice?.lang ?? (settings.voice.startsWith("zh") ? "zh-CN" : "en-US");
      utter.rate = Math.min(10, Math.max(0.1, settings.rate));
      utter.pitch = Math.min(2, Math.max(0, settings.pitch));
      utter.volume = Math.min(1, Math.max(0, settings.volume / 100));
      this.currentUtterance = utter;
      let settled = false;
      utter.onend = () => {
        if (!settled) {
          settled = true;
          if (this.currentUtterance === utter) this.currentUtterance = null;
          resolve();
        }
      };
      utter.onerror = (e) => {
        if (!settled && e.error !== "interrupted" && e.error !== "canceled") {
          settled = true;
          if (this.currentUtterance === utter) this.currentUtterance = null;
          reject(new Error("\u7CFB\u7EDF\u8BED\u97F3\u64AD\u653E\u5931\u8D25"));
        }
      };
      window.speechSynthesis.speak(utter);
    });
  }
  pause() {
    if (this._state.value !== "playing") return;
    this.paused = true;
    this._state.value = "paused";
    this.audio?.pause();
    window.speechSynthesis?.pause();
  }
  resume() {
    if (this._state.value !== "paused") return;
    this.paused = false;
    this._state.value = "playing";
    this.audio?.play();
    window.speechSynthesis?.resume();
  }
  /** 停止并清空队列（completed=false 的 onFinish 已在 start 循环中处理） */
  stop(silent = false) {
    this.token += 1;
    this.paused = false;
    this._state.value = "idle";
    this.currentIndex.value = -1;
    if (this.audio) {
      this.audio.pause();
      this.audio = null;
    }
    window.speechSynthesis?.cancel();
    this.currentUtterance = null;
    if (!silent) {
    }
  }
  sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}
const ttsPlayer = new TtsPlayer();
