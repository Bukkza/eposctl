"""Generates assets/eposctl.ico and assets/logo.png.

The two waveform silhouettes are drawn on a 32×32 grid. The app also uses
logo.png for the sidebar, window and tray. Standard library only:

    python assets/make_icon.py
"""

import math
import os
import struct
import zlib

WAVEFORMS = (
    (
        (0x83, 0xA8, 0xFF),
        (
            (2.5, 18.75), (6.5, 14.75), (8.5, 14.75), (18.25, 5),
            (21.5, 11.25), (23.25, 9.5), (29.75, 9.5), (25.5, 13.75),
            (18.5, 13.75), (16.75, 10.75), (9.25, 18.75),
        ),
    ),
    (
        (0x3D, 0x64, 0xED),
        (
            (2.25, 23.5), (6, 19.75), (11.5, 19.75), (13, 22.25),
            (18.5, 15.25), (28.5, 15.25), (25, 18.75), (20.5, 18.75),
            (12, 27), (9.5, 23), (9, 23.5),
        ),
    ),
)


def spans(points, y):
    crossings = []
    for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1]):
        if (y0 > y) != (y1 > y):
            crossings.append(x0 + (y - y0) * (x1 - x0) / (y1 - y0))
    crossings.sort()
    return zip(crossings[::2], crossings[1::2])


def rgba(size, ss=4):
    scale = size * ss / 32
    out = bytearray()
    for y in range(size):
        rows = []
        for sy in range(ss):
            row = [None] * (size * ss)
            sample_y = (y * ss + sy + 0.5) / scale
            for color, points in WAVEFORMS:
                for left, right in spans(points, sample_y):
                    start = math.ceil(left * scale - 0.5)
                    end = math.ceil(right * scale - 0.5)
                    row[start:end] = [color] * (end - start)
            rows.append(row)

        out.append(0)  # PNG filter byte
        for x in range(size):
            samples = [color for row in rows for color in row[x * ss:(x + 1) * ss] if color is not None]
            if samples:
                rgb = [round(sum(channel) / len(samples)) for channel in zip(*samples)]
                out += bytes(rgb + [round(len(samples) / (ss * ss) * 255)])
            else:
                out += b"\0\0\0\0"
    return bytes(out)


def png(size):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(rgba(size), 9)) + chunk(b"IEND", b"")


def ico(sizes):
    images = [png(s) for s in sizes]
    head = struct.pack("<HHH", 0, 1, len(sizes))
    offset = 6 + 16 * len(sizes)
    entries = b""
    for s, data in zip(sizes, images):
        entries += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    return head + entries + b"".join(images)


here = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(here, "eposctl.ico"), "wb") as f:
    f.write(ico([16, 20, 24, 32, 40, 48, 64, 128, 256]))
with open(os.path.join(here, "logo.png"), "wb") as f:
    f.write(png(160))
print("ok")
