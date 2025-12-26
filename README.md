# minmpeg

A minimal video generation FFI library written in Rust. Designed for use from Go (CGO) with a simple API.

[日本語版 README](README.ja.md)

## Features

- **slideshow**: Create video from a sequence of images
- **juxtapose**: Combine two videos side by side
- **available**: Check codec availability

## Supported Formats

| Container | Supported Codecs | Notes |
|-----------|------------------|-------|
| MP4 | H.264 | AV1 not supported due to mp4 crate limitations |
| WebM | AV1 | |

### Codec Implementations

| Codec | Implementation |
|-------|----------------|
| AV1 | rav1e (all platforms) |
| H.264 | Platform-dependent (see below) |

### H.264 Encoder by Platform

| Platform | Implementation |
|----------|----------------|
| macOS | VideoToolbox (OS native) |
| Windows | Media Foundation (OS native) |
| Linux | ffmpeg (external process) |

## Installation

### Build Requirements

- Rust 1.80+
- Cargo

#### Platform-specific

| Platform | Additional Requirements |
|----------|------------------------|
| macOS | Xcode Command Line Tools |
| Windows | Visual Studio Build Tools |
| Linux | nasm |

### Build

```bash
# Debug build
make build

# Release build
make build-release
```

### Test

```bash
# Rust tests
make test

# Go bindings test (includes release build)
make test-golang

# All tests
make test-all
```

## Usage

### Go Bindings

```go
package main

import (
    minmpeg "github.com/ideamans/rust-minmpeg/golang"
)

func main() {
    // Check codec availability
    if err := minmpeg.Available(minmpeg.CodecAV1, ""); err != nil {
        panic(err)
    }

    // Create slideshow
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
        50, // quality (0-100)
        "", // ffmpeg path (empty = search PATH)
    )
    if err != nil {
        panic(err)
    }
}
```

### C/C++ API

See [include/minmpeg.h](include/minmpeg.h) for the full API.

```c
#include "minmpeg.h"

// Check codec availability
Result result = minmpeg_available(CODEC_AV1, NULL);
if (result.code != MINMPEG_OK) {
    printf("Error: %s\n", result.message);
    minmpeg_free_result(&result);
}

// Create slideshow
SlideEntry entries[] = {
    {"slide1.png", 2000},
    {"slide2.png", 2000},
};

result = minmpeg_slideshow(
    entries, 2,
    "output.webm",
    CONTAINER_WEBM,
    CODEC_AV1,
    50,   // quality
    NULL  // ffmpeg path
);
minmpeg_free_result(&result);
```

## API Reference

### Functions

#### `minmpeg_available`
Check if a codec is available on the current system.

#### `minmpeg_slideshow`
Create a video from a sequence of images.
- Supported image formats: JPEG, PNG, WebP, GIF (static)
- Duration specified in milliseconds per image
- Images are resized to match the first image's dimensions

#### `minmpeg_juxtapose`
Combine two videos side by side.
- Different durations: shorter video holds its last frame
- Different heights: videos are top-aligned, bottom padded with background color
- Frame rate: inherits from input (uses higher rate if different)

### Quality Mapping

| Codec | Quality 0-100 | Internal |
|-------|---------------|----------|
| AV1 | 0-100 → CRF 63-0 | Default: 50 (CRF 31) |
| H.264 | 0-100 → CRF 51-0 | Default: 50 (CRF 23) |

### Container/Codec Compatibility

| Container | AV1 | H.264 |
|-----------|-----|-------|
| MP4 | NG | OK |
| WebM | OK | NG |

## CI/CD

### Test Platforms

| OS | Architecture |
|----|--------------|
| macOS | ARM64 |
| Linux | ARM64, AMD64 |
| Windows | AMD64 |

### Releases

Tagged releases (`v*`) automatically build and publish:
- `minmpeg-macos-arm64.tar.gz`
- `minmpeg-linux-amd64.tar.gz`
- `minmpeg-linux-arm64.tar.gz`
- `minmpeg-windows-amd64.zip`

Each archive contains:
- Static library (`libminmpeg.a` or `minmpeg.lib`)
- Header file (`minmpeg.h`)

## License

MIT License

### License Rationale

- rav1e: BSD-2-Clause (MIT compatible)
- VideoToolbox (macOS): Proprietary but link-only
- Media Foundation (Windows): Proprietary but link-only
- ffmpeg (Linux): External process call, no GPL contamination

To avoid GPL contamination:
- No GPL libraries (like x264) are linked
- ffmpeg is called as external process, not linked
