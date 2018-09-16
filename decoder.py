#!/usr/bin/env python3

from sys import argv, stdout
import cv2
import numpy as np
from test_image_encode import decode_frame, actual_size, virtual_size, pi_frames


def main():
    assert(len(argv) == 3)
    cap = cv2.VideoCapture(int(argv[1]))
    dp = 0

    data_length = int(argv[2])

    reconstructed_data = np.zeros((data_length,), dtype=np.uint8)

    # Phase offset
    if cap.isOpened():
        for i in range(pi_frames // 2):
            ret, frame = cap.read()

    while (cap.isOpened()):
        data_frame = decode_frame(frame, virtual_size)
        end_ptr = dp + data_frame.size
        #print(dp, end_ptr, data_frame.size)
        if end_ptr < data_length:
            reconstructed_data[dp:end_ptr] = data_frame
        else:
            reconstructed_data[dp:] = data_frame[:len(
                reconstructed_data) - dp]
            break
        dp = end_ptr

        for i in range(pi_frames):
            ret, frame = cap.read()
    stdout.buffer.write(reconstructed_data.tobytes())


if __name__ == '__main__':
    main()
