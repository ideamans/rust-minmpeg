package minmpeg

import (
	"fmt"
	"image"
	"image/color"
	"image/png"
	"os"
	"path/filepath"
	"testing"
)

// createTestImage creates a simple colored PNG image for testing
func createTestImage(path string, width, height int, c color.Color) error {
	img := image.NewRGBA(image.Rect(0, 0, width, height))

	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			img.Set(x, y, c)
		}
	}

	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()

	return png.Encode(f, img)
}

// verifyWebMHeader checks if a file starts with valid WebM/EBML header
func verifyWebMHeader(path string) bool {
	f, err := os.Open(path)
	if err != nil {
		return false
	}
	defer f.Close()

	header := make([]byte, 4)
	if _, err := f.Read(header); err != nil {
		return false
	}

	// WebM starts with EBML header: 0x1A 0x45 0xDF 0xA3
	return header[0] == 0x1A && header[1] == 0x45 && header[2] == 0xDF && header[3] == 0xA3
}

func TestAvailable(t *testing.T) {
	err := Available(CodecAV1, "")
	if err != nil {
		t.Errorf("AV1 codec should be available: %v", err)
	}
}

func TestSlideshowCreatesValidVideo(t *testing.T) {
	// Create temp directory
	tmpDir, err := os.MkdirTemp("", "minmpeg-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	// Create test images
	colors := []color.Color{
		color.RGBA{255, 0, 0, 255},   // Red
		color.RGBA{0, 255, 0, 255},   // Green
		color.RGBA{0, 0, 255, 255},   // Blue
	}

	entries := make([]SlideEntry, len(colors))
	for i, c := range colors {
		imgPath := filepath.Join(tmpDir, fmt.Sprintf("slide_%d.png", i))
		if err := createTestImage(imgPath, 320, 240, c); err != nil {
			t.Fatalf("Failed to create test image: %v", err)
		}
		entries[i] = SlideEntry{
			Path:       imgPath,
			DurationMs: 500, // 0.5 seconds each
		}
	}

	// Create slideshow
	outputPath := filepath.Join(tmpDir, "output.webm")
	err = Slideshow(entries, outputPath, ContainerWebM, CodecAV1, 50, "")
	if err != nil {
		t.Fatalf("Slideshow failed: %v", err)
	}

	// Verify output file exists
	info, err := os.Stat(outputPath)
	if err != nil {
		t.Fatalf("Output file does not exist: %v", err)
	}

	// Verify file is not empty
	if info.Size() == 0 {
		t.Fatal("Output file is empty")
	}

	// Verify it's a valid WebM file
	if !verifyWebMHeader(outputPath) {
		t.Fatal("Output file is not a valid WebM")
	}

	t.Logf("Created valid WebM file: %s (%d bytes)", outputPath, info.Size())
}

func TestVersion(t *testing.T) {
	version := Version()
	if version == "" {
		t.Error("Version should not be empty")
	}
	t.Logf("Library version: %s", version)
}
