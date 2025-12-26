/**
 * minmpeg - Minimal video generation library
 *
 * This library provides two main functions:
 * - slideshow: Create a video from a sequence of images
 * - juxtapose: Combine two videos side by side
 */

#ifndef MINMPEG_H
#define MINMPEG_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Container format types
 */
typedef enum {
    CONTAINER_MP4 = 0,
    CONTAINER_WEBM = 1,
} Container;

/**
 * Video codec types
 */
typedef enum {
    CODEC_AV1 = 0,
    CODEC_H264 = 1,
} Codec;

/**
 * Error codes
 */
typedef enum {
    MINMPEG_OK = 0,
    MINMPEG_ERR_INVALID_INPUT = 1,
    MINMPEG_ERR_CODEC_UNAVAILABLE = 2,
    MINMPEG_ERR_CONTAINER_CODEC_MISMATCH = 3,
    MINMPEG_ERR_IO_ERROR = 4,
    MINMPEG_ERR_ENCODE_ERROR = 5,
    MINMPEG_ERR_DECODE_ERROR = 6,
} ErrorCode;

/**
 * Result structure returned by minmpeg functions
 */
typedef struct {
    ErrorCode code;
    char* message;  /* Error message (must be freed with minmpeg_free_result) */
} Result;

/**
 * Slide entry for slideshow creation
 */
typedef struct {
    const char* path;      /* Path to the image file */
    uint32_t duration_ms;  /* Duration to display this image in milliseconds */
} SlideEntry;

/**
 * RGB color
 */
typedef struct {
    uint8_t r;
    uint8_t g;
    uint8_t b;
} Color;

/**
 * Check if a codec is available on this system
 *
 * @param codec         The codec to check
 * @param ffmpeg_path   Optional path to ffmpeg executable (for H.264 on Linux)
 *                      Pass NULL to search in PATH
 * @return              Result with code MINMPEG_OK if available
 */
Result minmpeg_available(Codec codec, const char* ffmpeg_path);

/**
 * Create a slideshow video from a sequence of images
 *
 * All images are resized to match the dimensions of the first image.
 * The output video frame rate is 30 fps.
 *
 * @param entries       Array of slide entries
 * @param entry_count   Number of entries in the array
 * @param output_path   Path to the output video file
 * @param container     Container format (MP4 or WebM)
 * @param codec         Video codec (AV1 or H264)
 * @param quality       Quality (0-100, where 100 is highest quality)
 * @param ffmpeg_path   Optional path to ffmpeg (for H.264 on Linux), NULL for PATH
 * @return              Result with code MINMPEG_OK on success
 */
Result minmpeg_slideshow(
    const SlideEntry* entries,
    size_t entry_count,
    const char* output_path,
    Container container,
    Codec codec,
    uint8_t quality,
    const char* ffmpeg_path
);

/**
 * Combine two videos side by side
 *
 * The output video will have:
 * - Width = left video width + right video width
 * - Height = max(left video height, right video height)
 * - Duration = max(left video duration, right video duration)
 *
 * If heights differ, videos are aligned to the top with background filling bottom.
 * If durations differ, the shorter video shows its last frame until the end.
 *
 * @param left_path     Path to the left video file
 * @param right_path    Path to the right video file
 * @param output_path   Path to the output video file
 * @param container     Container format (MP4 or WebM)
 * @param codec         Video codec (AV1 or H264)
 * @param quality       Quality (0-100, where 100 is highest quality)
 * @param background    Background color for padding (NULL for white)
 * @param ffmpeg_path   Optional path to ffmpeg, NULL for PATH
 * @return              Result with code MINMPEG_OK on success
 */
Result minmpeg_juxtapose(
    const char* left_path,
    const char* right_path,
    const char* output_path,
    Container container,
    Codec codec,
    uint8_t quality,
    const Color* background,
    const char* ffmpeg_path
);

/**
 * Free resources associated with a Result
 *
 * This function must be called to free the error message string.
 *
 * @param result    Pointer to the result to free
 */
void minmpeg_free_result(Result* result);

/**
 * Get the library version string
 *
 * @return  Version string (e.g., "0.1.0")
 */
const char* minmpeg_version(void);

#ifdef __cplusplus
}
#endif

#endif /* MINMPEG_H */
