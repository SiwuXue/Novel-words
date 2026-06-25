use printpdf::ParsedFont;
use std::path::Path;

/// Try to find a Chinese-capable font file on the system.
pub fn find_chinese_font() -> Option<String> {
    #[cfg(target_os = "windows")]
    let candidates = [
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simkai.ttf",
        r"C:\Windows\Fonts\FZSTK.TTF",
        r"C:\Windows\Fonts\FZKTK.TTF",
        r"C:\Windows\Fonts\SIMLI.TTF",
        r"C:\Windows\Fonts\FZYTK.TTF",
        r"C:\Windows\Fonts\arialuni.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\mingliu.ttc",
    ];
    #[cfg(target_os = "macos")]
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STSong.ttf",
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ];

    for path in &candidates {
        if Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    None
}
