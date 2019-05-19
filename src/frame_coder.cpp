#include "Halide.h"

#include <stdio.h>

using namespace Halide;

extern "C" {
    const unsigned int FRAME_LEN = 1024 * 10;

    typedef unsigned char BitString[FRAME_LEN];

    struct ImageBuffer {
        short width;
        short height;
        unsigned char* data;
    };
}

extern "C" {
    ImageBuffer* create_image_buffer(const short width,
                                     const short height,
                                     unsigned char* data) {
        return new ImageBuffer {width, height, data};
    }

    void release_resources(ImageBuffer* buffer) {
        delete buffer->data;
    }

    void delete_handle(ImageBuffer* buffer) {
        delete buffer;
    }
}

extern "C" {
    void encode(const BitString input,
                ImageBuffer* write_img) {

    }

    void decode(BitString output,
                const ImageBuffer* read_img) {

    }
}
