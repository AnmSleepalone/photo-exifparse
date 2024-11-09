#include <chrono>
#include <iostream>
#include <vector>
#include <fstream>
#include <memory>
#include <thread>
#include <exception>
#include <turbojpeg.h>
#ifdef __x86_64__
#include <immintrin.h>
#endif

#ifdef __aarch64__
#include <arm_neon.h>
#endif


// Image dimensions constants
constexpr uint32_t ORIGINAL_WIDTH = 19000;
constexpr uint32_t ORIGINAL_HEIGHT = 19000;
constexpr uint32_t NEW_WIDTH = 20000;
constexpr uint32_t NEW_HEIGHT = 20000;
constexpr uint32_t OFFSET_X = 500;
constexpr uint32_t OFFSET_Y = 500;
constexpr uint8_t JPEG_QUALITY = 100;
constexpr uint32_t BYTES_PER_PIXEL = 3;

#ifdef __x86_64__
constexpr size_t SIMD_VECTOR_SIZE = 32; // 256-bit AVX2
#elif defined(__aarch64__)
constexpr size_t SIMD_VECTOR_SIZE = 16; // 128-bit NEON
#else
constexpr size_t SIMD_VECTOR_SIZE = 1;
#endif

constexpr size_t MIN_BYTES_FOR_SIMD = SIMD_VECTOR_SIZE * 2;

enum class SimdSupport {
    None,
    AVX2,
    SSE4,
    NEON
};

struct BenchmarkResults {
    std::chrono::duration<double> copy_time;
    std::chrono::duration<double> encode_time;
    std::chrono::duration<double> total_time;
};

// Helper function to check SIMD support
SimdSupport get_simd_support() {
#ifdef __x86_64__
#ifdef __AVX2__
    return SimdSupport::AVX2;
#elif defined(__SSE4_1__)
    return SimdSupport::SSE4;
#else
    return SimdSupport::None;
#endif
#elif defined(__aarch64__)
    return SimdSupport::NEON;
#else
    return SimdSupport::None;
#endif
}

// SIMD implementations
#ifdef __aarch64__
inline void process_pixels_neon(uint8_t* pixels, size_t len, float brightness) {
    float32x4_t brightness_f32 = vdupq_n_f32(brightness);
    size_t i = 0;

    while (i + 16 <= len) {
        uint8x16_t pixels_u8 = vld1q_u8(pixels + i);

        // Process low half
        uint16x8_t pixels_u16_low = vmovl_u8(vget_low_u8(pixels_u8));
        uint32x4_t pixels_u32_low = vmovl_u16(vget_low_u16(pixels_u16_low));
        float32x4_t pixels_f32_low = vcvtq_f32_u32(pixels_u32_low);
        pixels_f32_low = vmulq_f32(pixels_f32_low, brightness_f32);

        // Process high half
        uint16x8_t pixels_u16_high = vmovl_u8(vget_high_u8(pixels_u8));
        uint32x4_t pixels_u32_high = vmovl_u16(vget_low_u16(pixels_u16_high));
        float32x4_t pixels_f32_high = vcvtq_f32_u32(pixels_u32_high);
        pixels_f32_high = vmulq_f32(pixels_f32_high, brightness_f32);

        // Convert back to u8
        uint32x4_t result_u32_low = vcvtq_u32_f32(pixels_f32_low);
        uint32x4_t result_u32_high = vcvtq_u32_f32(pixels_f32_high);
        uint16x8_t result_u16 = vcombine_u16(vqmovn_u32(result_u32_low),
            vqmovn_u32(result_u32_high));
        uint8x8_t result_u8 = vqmovn_u16(result_u16);

        vst1_u8(pixels + i, result_u8);
        i += 16;
    }

    // Handle remaining pixels
    while (i < len) {
        float val = pixels[i] * brightness;
        pixels[i] = static_cast<uint8_t>(std::min(255.0f, std::max(0.0f, val)));
        i++;
    }
}
#endif

#ifdef __x86_64__
inline void process_pixels_avx2(uint8_t* pixels, size_t len, float brightness) {
    __m256 brightness_factor = _mm256_set1_ps(brightness);
    size_t i = 0;

    while (i + 32 <= len) {
        __m256i pixels_avx = _mm256_loadu_si256(
            reinterpret_cast<const __m256i*>(pixels + i));

        // Process low 16 bytes
        __m256i pixels_low = _mm256_cvtepu8_epi32(
            _mm_loadu_si128(reinterpret_cast<const __m128i*>(pixels + i)));
        __m256 float_low = _mm256_cvtepi32_ps(pixels_low);
        __m256 processed_low = _mm256_mul_ps(float_low, brightness_factor);

        // Process high 16 bytes
        __m256i pixels_high = _mm256_cvtepu8_epi32(
            _mm_loadu_si128(reinterpret_cast<const __m128i*>(pixels + i + 16)));
        __m256 float_high = _mm256_cvtepi32_ps(pixels_high);
        __m256 processed_high = _mm256_mul_ps(float_high, brightness_factor);

        // Convert back and combine
        __m256i result_low = _mm256_cvtps_epi32(processed_low);
        __m256i result_high = _mm256_cvtps_epi32(processed_high);
        __m256i result = _mm256_packus_epi32(result_low, result_high);
        __m256i final_result = _mm256_packus_epi16(result, result);

        _mm256_storeu_si256(reinterpret_cast<__m256i*>(pixels + i), final_result);
        i += 32;
    }

    // Handle remaining pixels
    while (i < len) {
        float val = pixels[i] * brightness;
        pixels[i] = static_cast<uint8_t>(std::min(255.0f, std::max(0.0f, val)));
        i++;
    }
}
#endif

inline void copy_row_simd(const uint8_t* src, uint8_t* dst, size_t width, SimdSupport simd_support) {
    switch (simd_support) {
#ifdef __aarch64__
    case SimdSupport::NEON: {
        size_t i = 0;
        while (i + 16 <= width) {
            uint8x16_t data = vld1q_u8(src + i);
            vst1q_u8(dst + i, data);
            i += 16;
        }
        std::memcpy(dst + i, src + i, width - i);
        break;
    }
#endif

#ifdef __x86_64__
    case SimdSupport::AVX2: {
        size_t i = 0;
        while (i + 32 <= width) {
            __m256i data = _mm256_loadu_si256(
                reinterpret_cast<const __m256i*>(src + i));
            _mm256_storeu_si256(
                reinterpret_cast<__m256i*>(dst + i), data);
            i += 32;
        }
        std::memcpy(dst + i, src + i, width - i);
        break;
    }
#endif

    default:
        std::memcpy(dst, src, width);
    }
}

class ImageProcessor {
private:
    std::vector<uint8_t> original_img;
    std::vector<uint8_t> new_img;
    SimdSupport simd_support;

public:
    ImageProcessor() :
        original_img(ORIGINAL_WIDTH* ORIGINAL_HEIGHT* BYTES_PER_PIXEL),
        new_img(NEW_WIDTH* NEW_HEIGHT* BYTES_PER_PIXEL),
        simd_support(get_simd_support()) {

        // Initialize original image with gray color
        std::fill(original_img.begin(), original_img.end(), 128);
    }

    BenchmarkResults process_image() {
        auto start_time = std::chrono::high_resolution_clock::now();

        // Start copy operation
        auto copy_start = std::chrono::high_resolution_clock::now();

        const size_t row_bytes = ORIGINAL_WIDTH * BYTES_PER_PIXEL;
        const size_t offset_bytes = OFFSET_X * BYTES_PER_PIXEL;
        const size_t new_row_bytes = NEW_WIDTH * BYTES_PER_PIXEL;

        // Parallel processing
        const size_t num_threads = std::thread::hardware_concurrency();
        std::vector<std::thread> threads;

        auto process_chunk = [&](size_t start_y, size_t end_y) {
            for (size_t y = start_y; y < end_y; ++y) {
                if (y >= OFFSET_Y && y < (OFFSET_Y + ORIGINAL_HEIGHT)) {
                    size_t src_y = y - OFFSET_Y;
                    if (src_y < ORIGINAL_HEIGHT) {
                        const uint8_t* src_row = original_img.data() +
                            (src_y * row_bytes);
                        uint8_t* dst_row = new_img.data() +
                            (y * new_row_bytes) + offset_bytes;

                        copy_row_simd(src_row, dst_row, row_bytes, simd_support);
                    }
                }
            }
            };

        size_t rows_per_thread = NEW_HEIGHT / num_threads;
        for (size_t i = 0; i < num_threads; ++i) {
            size_t start_y = i * rows_per_thread;
            size_t end_y = (i == num_threads - 1) ? NEW_HEIGHT :
                (i + 1) * rows_per_thread;
            threads.emplace_back(process_chunk, start_y, end_y);
        }

        for (auto& thread : threads) {
            thread.join();
        }

        auto copy_time = std::chrono::high_resolution_clock::now() - copy_start;

        // Start JPEG encoding
        auto encode_start = std::chrono::high_resolution_clock::now();

        // Initialize TurboJPEG
        tjhandle jpeg_compressor = tjInitCompress();
        if (!jpeg_compressor) {
            throw std::runtime_error("Failed to initialize TurboJPEG compressor");
        }

        unsigned char* jpeg_buffer = nullptr;
        unsigned long jpeg_size = 0;

        int result = tjCompress2(jpeg_compressor,
            new_img.data(),
            NEW_WIDTH,
            0, // pitch
            NEW_HEIGHT,
            TJPF_RGB,
            &jpeg_buffer,
            &jpeg_size,
            TJSAMP_444,
            JPEG_QUALITY,
            TJFLAG_FASTDCT);

        if (result != 0) {
            tjDestroy(jpeg_compressor);
            throw std::runtime_error("JPEG compression failed");
        }

        // Write to file
        std::ofstream outfile("output.jpg", std::ios::binary);
        outfile.write(reinterpret_cast<char*>(jpeg_buffer), jpeg_size);
        outfile.close();

        tjFree(jpeg_buffer);
        tjDestroy(jpeg_compressor);

        auto encode_time = std::chrono::high_resolution_clock::now() - encode_start;
        auto total_time = std::chrono::high_resolution_clock::now() - start_time;

        return BenchmarkResults{
            copy_time,
            encode_time,
            total_time
        };
    }
};

int main() {
    std::cout << "Architecture: " <<
#ifdef __x86_64__
        "x86_64"
#elif defined(__aarch64__)
        "aarch64"
#else
        "unknown"
#endif
        << std::endl;

    std::cout << "SIMD Support: ";
    switch (get_simd_support()) {
    case SimdSupport::AVX2:
        std::cout << "AVX2";
        break;
    case SimdSupport::SSE4:
        std::cout << "SSE4";
        break;
    case SimdSupport::NEON:
        std::cout << "NEON";
        break;
    default:
        std::cout << "None";
    }
    std::cout << std::endl;

    const int iterations = 3;
    std::vector<BenchmarkResults> total_results;
    total_results.reserve(iterations);

    std::cout << "\nRunning " << iterations << " iterations..." << std::endl;

    try {
        ImageProcessor processor;

        for (int i = 0; i < iterations; ++i) {
            std::cout << "\nIteration " << (i + 1) << ":" << std::endl;
            auto results = processor.process_image();

            std::cout << "  Copy time: " <<
                std::chrono::duration_cast<std::chrono::milliseconds>(
                    results.copy_time).count() << "ms" << std::endl;
            std::cout << "  Encode time: " <<
                std::chrono::duration_cast<std::chrono::milliseconds>(
                    results.encode_time).count() << "ms" << std::endl;
            std::cout << "  Total time: " <<
                std::chrono::duration_cast<std::chrono::milliseconds>(
                    results.total_time).count() << "ms" << std::endl;

            total_results.push_back(results);
        }

        // Calculate averages
        std::chrono::duration<double> avg_copy(0), avg_encode(0), avg_total(0);
        for (const auto& result : total_results) {
            avg_copy += result.copy_time;
            avg_encode += result.encode_time;
            avg_total += result.total_time;
        }
    }
    catch (std::exception  e) {
        std::cout << "异常出现了" << std::endl;
    };
}