#!/usr/bin/env python3

import numpy as np
import cv2

pi_frames = 3

actual_size = (1920, 1080)
virtual_size = (2, 2)


def encode_pixel(datum):
    hue_bits = datum % (2 ** 2)
    saturation_bits = (datum >> 2) % (2 ** 2)
    value_bits = (datum >> 4) % (2 ** 4)

    return (value_bits * 16,
            saturation_bits * 64,
            hue_bits * (180 // 4))


def decode_pixel(value, saturation, hue):
    value_rounded = int(value / 16)
    saturation_rounded = int(saturation / 64)
    hue_rounded = int(hue / (180 // 4))
    return (value_rounded << 4) + (saturation_rounded << 2) + hue_rounded


def encode_frame(data, virtual_resolution, actual_resolution):
    indices = (actual_resolution[0] / virtual_resolution[0],
               actual_resolution[1] / virtual_resolution[1])
    img = np.zeros((actual_resolution[1], actual_resolution[0], 3),
                   dtype=np.uint8)
    data_index = 0

    for ix in range(0, virtual_resolution[0]):
        for iy in range(0, virtual_resolution[1]):
            if data_index >= len(data):
                break
            vx = int(ix * indices[0])
            vy = int(iy * indices[1])
            datum = data[data_index]
            value, saturation, hue = encode_pixel(datum)

            img[vy:int(vy+indices[1]), vx:int(vx+indices[0]), 0] = hue
            img[vy:int(vy+indices[1]), vx:int(vx+indices[0]), 1] = saturation
            img[vy:int(vy+indices[1]), vx:int(vx+indices[0]), 2] = value

            if data_index < 50:
                print("{}: {} {} {} {}".format(
                    data_index, hue, saturation, value, datum))

            data_index += 1
    rgb = cv2.cvtColor(img, cv2.COLOR_HSV2BGR)
    # cv2.imshow('rgb', rgb)
    # cv2.waitKey(10000)

    return (rgb, data_index)


def decode_frame(frame, virtual_resolution):
    hsv_frame = cv2.cvtColor(frame, cv2.COLOR_BGR2HSV)

    actual_resolution = frame.shape[:2]
    indices = (actual_resolution[1] / virtual_resolution[0],
               actual_resolution[0] / virtual_resolution[1])
    print("actual_resolution: {}".format(actual_resolution))
    img = np.zeros((actual_resolution[0], actual_resolution[1], 3),
                   dtype=np.uint8)
    data_index = 0

    data = np.zeros(
        (virtual_resolution[0] * virtual_resolution[1],), dtype=np.uint8)
    for ix in range(0, virtual_resolution[0]):
        for iy in range(0, virtual_resolution[1]):
            vx = int(ix * indices[0])
            vy = int(iy * indices[1])
            vx_1 = min(vx+int(indices[0]), actual_resolution[1] - 1)
            vy_1 = min(vy+int(indices[1]), actual_resolution[0] - 1)
            patch = hsv_frame[vy:vy_1, vx:vx_1]
            print(vx, vy, vx_1, vy_1)
            print(patch.shape)
            cv2.imshow('frame', cv2.cvtColor(hsv_frame, cv2.COLOR_HSV2BGR))
            cv2.imshow('patch', cv2.cvtColor(patch, cv2.COLOR_HSV2BGR))
            cv2.waitKey(0)
            avg_color = np.sum(patch, axis=(0, 1)) \
                / (indices[0] * indices[1])
            #print("{} vs {}".format(avg_color, hsv_frame[vy+5, vx+5]))
            (value, saturation, hue) = np.round(avg_color).astype(np.uint8)
            datum = decode_pixel(value, saturation, hue)
            data[data_index] = datum

            if data_index < 50:
                print("{}: {} {} {} {}".format(
                    data_index, hue, saturation, value, datum))

            data_index += 1
    return data


def encode_video(data, out):
    dp = 0

    a = 0
    while True:
        frame, dp = encode_frame(data, virtual_size, actual_size)
        for i in range(pi_frames):
            #print("New frame {}".format(a))
            out.write(frame)
            a += 1
        if dp >= len(data):
            break
        data = data[dp:]
    out.release()


def decode_video(cap, data_length):
    reconstructed_data = np.zeros((data_length,), dtype=np.uint8)
    dp = 0
    i = 0

    if cap.isOpened():
        for i in range(pi_frames // 2):
            ret, frame = cap.read()

    while (cap.isOpened()):
        # cv2.imshow(frame)
        # cv2.waitKey(0)
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

        for i in range(pi_frames - 1):
            ret, frame = cap.read()
    with open('output.txt', 'w') as output_fp:
        output_fp.write((reconstructed_data % 128).tostring().decode('ascii'))

    return reconstructed_data


def test_encode_decode():
    for value in range(0, 256, 4):
        for saturation in range(0, 256, 4):
            for hue in range(0, 180, 5):
                all_should_be_the_same = []
                for dx in range(4):
                    for dy in range(4):
                        for dz in range(4):
                            decoded = decode_pixel(
                                value + dx, saturation + dy, hue + dz)
                            all_should_be_the_same.append(decoded)
                print(all_should_be_the_same)
                assert(len(set(all_should_be_the_same)) == 1)


def main():
    with open('data.txt', 'r') as fp:
        data = fp.read().encode('ascii')
        original_data = np.frombuffer(data[:], dtype=np.uint8)
        total_data_length = len(data)

        out = cv2.VideoWriter('test.avi', cv2.VideoWriter_fourcc(
            'X', '2', '6', '4'), 15, actual_size)
        cap = cv2.VideoCapture('test.avi')

        encode_video(data, out)
        decode_video(cap, total_data_length)


if __name__ == '__main__':
    for i in range(256):
        assert(i == decode_pixel(*encode_pixel(i)))
    # test_encode_decode()
    main()
