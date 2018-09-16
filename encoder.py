#!/usr/bin/env python3

from sys import argv, stdin
import cv2
from test_image_encode import encode_frame, actual_size, virtual_size, pi_frames


def main():
    assert(len(argv) == 2)
    out = cv2.VideoWriter(argv[1], cv2.VideoWriter_fourcc(
        'X', '2', '6', '4'), 15, actual_size)
    while True:
        data = stdin.buffer.read()
        img, dp = encode_frame(data, virtual_size, actual_size)
        print(dp, len(data))
        assert(dp >= len(data))
        for i in range(pi_frames):
            out.write(img)
    out.release()


if __name__ == '__main__':
    main()
