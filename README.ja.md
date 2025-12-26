# minmpeg

Rustで書かれた軽量な動画生成FFIライブラリ。Go言語(CGO)からの利用を前提とし、シンプルなAPIで動画生成機能を提供します。

[English README](README.md)

## 機能

- **slideshow**: 画像シーケンスから動画を生成
- **juxtapose**: 2つの動画を横並びで結合
- **available**: コーデックの利用可能性チェック

## 対応フォーマット

| コンテナ | 対応コーデック | 備考 |
|----------|----------------|------|
| MP4 | H.264 | mp4クレートの制約によりAV1は未対応 |
| WebM | AV1 | |

### コーデック実装

| コーデック | 実装 |
|------------|------|
| AV1 | rav1e (全プラットフォーム共通) |
| H.264 | プラットフォーム依存 (下記参照) |

### H.264エンコーダー (プラットフォーム別)

| プラットフォーム | 実装 |
|------------------|------|
| macOS | VideoToolbox (OS標準機能) |
| Windows | Media Foundation (OS標準機能) |
| Linux | ffmpeg (外部プロセス) |

## インストール

### ビルド要件

- Rust 1.80+
- Cargo

#### プラットフォーム別

| プラットフォーム | 追加要件 |
|------------------|----------|
| macOS | Xcode Command Line Tools |
| Windows | Visual Studio Build Tools |
| Linux | nasm |

### ビルド

```bash
# デバッグビルド
make build

# リリースビルド
make build-release
```

### テスト

```bash
# Rustテスト
make test

# Goバインディングテスト (リリースビルド含む)
make test-golang

# 全テスト
make test-all
```

## 使い方

### Goバインディング

```go
package main

import (
    minmpeg "github.com/ideamans/rust-minmpeg/golang"
)

func main() {
    // コーデックの利用可能性チェック
    if err := minmpeg.Available(minmpeg.CodecAV1, ""); err != nil {
        panic(err)
    }

    // スライドショー作成
    entries := []minmpeg.SlideEntry{
        {Path: "slide1.png", DurationMs: 2000},
        {Path: "slide2.png", DurationMs: 2000},
        {Path: "slide3.png", DurationMs: 2000},
    }

    err := minmpeg.Slideshow(
        entries,
        "output.webm",
        minmpeg.ContainerWebM,
        minmpeg.CodecAV1,
        50, // 品質 (0-100)
        "", // ffmpegパス (空=PATH検索)
    )
    if err != nil {
        panic(err)
    }
}
```

### C/C++ API

完全なAPIは [include/minmpeg.h](include/minmpeg.h) を参照してください。

```c
#include "minmpeg.h"

// コーデック利用可能性チェック
Result result = minmpeg_available(CODEC_AV1, NULL);
if (result.code != MINMPEG_OK) {
    printf("Error: %s\n", result.message);
    minmpeg_free_result(&result);
}

// スライドショー作成
SlideEntry entries[] = {
    {"slide1.png", 2000},
    {"slide2.png", 2000},
};

result = minmpeg_slideshow(
    entries, 2,
    "output.webm",
    CONTAINER_WEBM,
    CODEC_AV1,
    50,   // 品質
    NULL  // ffmpegパス
);
minmpeg_free_result(&result);
```

## APIリファレンス

### 関数

#### `minmpeg_available`
指定したコーデックが現在のシステムで利用可能かチェックします。

#### `minmpeg_slideshow`
画像シーケンスから動画を生成します。
- 対応画像形式: JPEG, PNG, WebP, GIF (静止画)
- 表示時間はミリ秒単位で指定
- 画像サイズが異なる場合、最初の画像サイズに統一（リサイズ）

#### `minmpeg_juxtapose`
2つの動画を横並びで結合します。
- 尺が異なる場合: 短い方は最終フレームを継続表示
- 高さが異なる場合: 上寄せで配置、下部を背景色で埋める
- フレームレート: 入力動画から継承（異なる場合は高い方を使用）

### 品質値マッピング

| コーデック | 品質 0-100 | 内部値 |
|------------|------------|--------|
| AV1 | 0-100 → CRF 63-0 | デフォルト: 50 (CRF 31相当) |
| H.264 | 0-100 → CRF 51-0 | デフォルト: 50 (CRF 23相当) |

### コンテナ/コーデック互換性

| コンテナ | AV1 | H.264 |
|----------|-----|-------|
| MP4 | NG | OK |
| WebM | OK | NG |

## CI/CD

### テスト対象プラットフォーム

| OS | アーキテクチャ |
|----|----------------|
| macOS | ARM64 |
| Linux | ARM64, AMD64 |
| Windows | AMD64 |

### リリース

`v*` タグで自動ビルド・公開:
- `minmpeg-macos-arm64.tar.gz`
- `minmpeg-linux-amd64.tar.gz`
- `minmpeg-linux-arm64.tar.gz`
- `minmpeg-windows-amd64.zip`

各アーカイブには以下が含まれます:
- 静的ライブラリ (`libminmpeg.a` または `minmpeg.lib`)
- ヘッダーファイル (`minmpeg.h`)

## ライセンス

MIT License

### ライセンス選択理由

- rav1e: BSD-2-Clause (MIT互換)
- VideoToolbox (macOS): プロプライエタリだがリンクのみ
- Media Foundation (Windows): プロプライエタリだがリンクのみ
- ffmpeg (Linux): 外部プロセス呼び出し、GPL汚染なし

GPL汚染を回避するため:
- x264等のGPLライブラリは使用しない
- ffmpegはライブラリリンクではなく外部プロセス呼び出し
