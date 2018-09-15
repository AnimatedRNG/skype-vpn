#!/usr/bin/env python3

import numpy as np
import cv2

pi_frames = 3


def encode_pixel(datum):
    #value = (datum // 4) * 4
    #hue = int((datum % 4) * (180 / 4))
    # return (value, hue)
    return (datum, 50)


def decode_pixel(value, hue):
    # return int(round(value + hue / (180 / 4)))
    return value


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
            value, hue = encode_pixel(datum)

            img[vy:int(vy+indices[1]), vx:int(vx+indices[0]), 0] = hue
            img[vy:int(vy+indices[1]), vx:int(vx+indices[0]), 1] = 128
            img[vy:int(vy+indices[1]), vx:int(vx+indices[0]), 2] = value

            data_index += 1
    rgb = cv2.cvtColor(img, cv2.COLOR_HSV2BGR)
    #cv2.imshow('rgb', rgb)
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
            #print(vx, vy, vx_1, vy_1)
            # print(patch.shape)
            #cv2.imshow('frame', cv2.cvtColor(hsv_frame, cv2.COLOR_HSV2BGR))
            #cv2.imshow('patch', cv2.cvtColor(patch, cv2.COLOR_HSV2BGR))
            # cv2.waitKey(0)
            avg_color = np.sum(patch, axis=(0, 1)) \
                / (indices[0] * indices[1])
            #print("{} vs {}".format(avg_color, hsv_frame[vy+5, vx+5]))
            (hue, test, value) = avg_color
            datum = decode_pixel(value, hue)
            data[data_index] = datum
            data_index += 1
    return data


def main():
    with open('data.txt', 'r') as fp:
        data = fp.read().encode('ascii')
        original_data = np.frombuffer(data[:], dtype=np.uint8)
        total_data_length = len(data)
        dp = 0

        actual_size = (1920, 1080)
        virtual_size = (2, 2)
        out = cv2.VideoWriter('test.avi', cv2.VideoWriter_fourcc(
            'X', '2', '6', '4'), 15, actual_size)
        last_frame = np.zeros(
            (actual_size[1], actual_size[0], 3), dtype=np.uint8)
        while True:
            frame, dp = encode_frame(data, virtual_size, actual_size)
            for i in range(1, pi_frames + 1):
                print("New frame")
                interp = frame * (i / 5) + last_frame * ((5 - i) / 5)
                out.write(interp.astype(np.uint8))
            if dp >= len(data):
                break
            data = data[dp:]
            last_frame = frame
        out.release()

        cap = cv2.VideoCapture('test.avi')

        reconstructed_data = np.zeros((total_data_length,), dtype=np.uint8)
        dp = 0
        while (cap.isOpened()):
            ret, frame = cap.read()
            for i in range(pi_frames - 1):
                cap.read()
            cv2.imshow('frame', frame)
            cv2.waitKey(0)

            data_frame = decode_frame(frame, virtual_size)
            end_ptr = dp + data_frame.size
            print(dp, end_ptr, data_frame.size)
            if end_ptr < total_data_length:
                reconstructed_data[dp:end_ptr] = data_frame
            else:
                reconstructed_data[dp:] = data_frame[:len(
                    reconstructed_data) - dp]
                break
            dp = end_ptr
        with open('output.txt', 'w') as output_fp:
            output_fp.write(reconstructed_data.tostring().decode('ascii'))
        print(sum(abs(reconstructed_data - original_data)))


if __name__ == '__main__':
    for i in range(256):
        assert(i == decode_pixel(*encode_pixel(i)))
    main()
