#!/usr/bin/env python3

from scipy import fftpack
import numpy as np
import cv2
import seaborn as sns
import matplotlib.pyplot as plt
sns.set(font_scale=2.0)


def plot(sdf, scale=[0, 255], center=0, fmt=".1f"):
    if scale:
        sns.heatmap(sdf, annot=True, fmt=fmt,
                    vmin=scale[0], vmax=scale[1], center=center,
                    cmap="Greys")
    else:
        sns.heatmap(sdf, annot=True, fmt=fmt,
                    center=center,
                    cmap="Greys")
    plt.show()


def index(i, j, width):
    return i * width + j


def unindex(ind, width):
    return (ind // width, ind % width)


def raster(dim=8):
    coords = []
    for i in range(dim):
        for j in range(i + 1):
            coords.append(index(i - j, j, dim))

    other_side = []
    for i in range(dim - 1):
        for j in range(i + 1):
            other_side.append(
                index((dim - 1) - (i - j), (dim - 1) - j, 8))
    other_side.reverse()
    return coords + other_side


def compress_bitstring(bitstring, bit_depth=2):
    packed = np.unpackbits(bitstring).reshape(-1, bit_depth)
    packed = np.pad(packed, (8 - bit_depth, 0),
                    mode='constant')
    unpacked = np.packbits(packed.ravel())

    return unpacked


def encode_block(unpacked, pattern,
                 bitstring_offset,
                 block_size=8,
                 density=0.3,
                 bit_depth=2):
    N = block_size * block_size
    block = np.zeros(N, np.float32)
    enc = int(N * density)

    offset = (2 ** (bit_depth - 1))
    data = unpacked[bitstring_offset:bitstring_offset +
                    enc].astype(np.int8) - offset

    block[pattern[:enc]] = data
    block = block.reshape(block_size, block_size)
    encoded_block = np.round(fftpack.idct(
        fftpack.idct(block, axis=0),
        axis=1))
    encoded_block += 128
    # plot(block, None)
    # plot(encoded_block)
    #assert((encoded_block >= 0).all() and (encoded_block <= 255).all())

    return encoded_block, bitstring_offset + enc


if __name__ == '__main__':
    output_img = np.zeros((1920, 1080), dtype=np.uint8)
    data = np.random.randint(0, 256, (200,), dtype=np.uint8)
    unpacked = compress_bitstring(data)
    bitstring_offset = 0

    for i in range(0, output_img.shape[0], 8):
        for j in range(0, output_img.shape[1], 8):
            encoded_block, bitstring_offset = \
                encode_block(unpacked,
                             pattern=raster(8),
                             bitstring_offset=bitstring_offset)

            if bitstring_offset >= unpacked.size:
                break
        else:
            continue
        break
