#!/usr/bin/bash

ffmpeg -f x11grab -r 15 -s $1x$2 -i :0.0+$3,$4 -vcodec rawvideo -pix_fmt rgb24 -threads 0 -f v4l2 /dev/video2
