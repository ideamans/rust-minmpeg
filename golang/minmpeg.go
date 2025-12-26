// Package minmpeg provides Go bindings for the minmpeg video generation library.
package minmpeg

/*
#cgo LDFLAGS: -L../target/release -lminmpeg
#cgo darwin LDFLAGS: -framework VideoToolbox -framework CoreMedia -framework CoreVideo -framework CoreFoundation -framework Security

#include "../include/minmpeg.h"
#include <stdlib.h>
*/
import "C"
import (
	"errors"
	"unsafe"
)

// Container represents video container formats
type Container int

const (
	ContainerMP4  Container = C.CONTAINER_MP4
	ContainerWebM Container = C.CONTAINER_WEBM
)

// Codec represents video codecs
type Codec int

const (
	CodecAV1  Codec = C.CODEC_AV1
	CodecH264 Codec = C.CODEC_H264
)

// Color represents an RGB color
type Color struct {
	R, G, B uint8
}

// SlideEntry represents a single slide in a slideshow
type SlideEntry struct {
	Path       string
	DurationMs uint32
}

// resultToError converts a C Result to a Go error
func resultToError(result C.Result) error {
	if result.code == C.MINMPEG_OK {
		return nil
	}

	var msg string
	if result.message != nil {
		msg = C.GoString(result.message)
		C.minmpeg_free_result(&result)
	} else {
		msg = "Unknown error"
	}

	return errors.New(msg)
}

// Available checks if a codec is available on this system
func Available(codec Codec, ffmpegPath string) error {
	var cPath *C.char
	if ffmpegPath != "" {
		cPath = C.CString(ffmpegPath)
		defer C.free(unsafe.Pointer(cPath))
	}

	result := C.minmpeg_available(C.Codec(codec), cPath)
	return resultToError(result)
}

// Slideshow creates a video from a sequence of images
func Slideshow(entries []SlideEntry, outputPath string, container Container, codec Codec, quality uint8, ffmpegPath string) error {
	if len(entries) == 0 {
		return errors.New("no slides provided")
	}

	// Convert entries
	cEntries := make([]C.SlideEntry, len(entries))
	cPaths := make([]*C.char, len(entries))

	for i, entry := range entries {
		cPaths[i] = C.CString(entry.Path)
		defer C.free(unsafe.Pointer(cPaths[i]))

		cEntries[i] = C.SlideEntry{
			path:        cPaths[i],
			duration_ms: C.uint32_t(entry.DurationMs),
		}
	}

	cOutputPath := C.CString(outputPath)
	defer C.free(unsafe.Pointer(cOutputPath))

	var cFfmpegPath *C.char
	if ffmpegPath != "" {
		cFfmpegPath = C.CString(ffmpegPath)
		defer C.free(unsafe.Pointer(cFfmpegPath))
	}

	result := C.minmpeg_slideshow(
		&cEntries[0],
		C.size_t(len(entries)),
		cOutputPath,
		C.Container(container),
		C.Codec(codec),
		C.uint8_t(quality),
		cFfmpegPath,
	)

	return resultToError(result)
}

// Juxtapose combines two videos side by side
func Juxtapose(leftPath, rightPath, outputPath string, container Container, codec Codec, quality uint8, background *Color, ffmpegPath string) error {
	cLeftPath := C.CString(leftPath)
	defer C.free(unsafe.Pointer(cLeftPath))

	cRightPath := C.CString(rightPath)
	defer C.free(unsafe.Pointer(cRightPath))

	cOutputPath := C.CString(outputPath)
	defer C.free(unsafe.Pointer(cOutputPath))

	var cBackground *C.Color
	if background != nil {
		bg := C.Color{
			r: C.uint8_t(background.R),
			g: C.uint8_t(background.G),
			b: C.uint8_t(background.B),
		}
		cBackground = &bg
	}

	var cFfmpegPath *C.char
	if ffmpegPath != "" {
		cFfmpegPath = C.CString(ffmpegPath)
		defer C.free(unsafe.Pointer(cFfmpegPath))
	}

	result := C.minmpeg_juxtapose(
		cLeftPath,
		cRightPath,
		cOutputPath,
		C.Container(container),
		C.Codec(codec),
		C.uint8_t(quality),
		cBackground,
		cFfmpegPath,
	)

	return resultToError(result)
}

// Version returns the library version string
func Version() string {
	return C.GoString(C.minmpeg_version())
}
