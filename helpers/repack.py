#!/usr/bin/env python3
"""
Repack a PNG into a roughly square image whose width and height are both powers of two.

Typical use case:
- input is a 1-pixel-tall PNG strip
- output is a texture atlas-like image
- pixels are copied left-to-right, top-to-bottom
- any unused pixels in the output are transparent

Requirements:
    pip install pillow

Usage:
    python repack_png_pow2.py input.png output.png
"""

from __future__ import annotations

import math
import sys
from pathlib import Path
from typing import Tuple

from PIL import Image


def is_power_of_two(n: int) -> bool:
    return n > 0 and (n & (n - 1)) == 0


def next_power_of_two(n: int) -> int:
    if n <= 1:
        return 1
    return 1 << (n - 1).bit_length()


def choose_pow2_dimensions(pixel_count: int) -> Tuple[int, int]:
    """
    Choose power-of-two width and height such that:
    - width * height >= pixel_count
    - shape is as close to square as possible

    Returns:
        (width, height)
    """
    if pixel_count <= 0:
        raise ValueError("pixel_count must be positive")

    total_area = next_power_of_two(pixel_count)
    min_exp = 0
    max_exp = int(math.log2(total_area))

    best = None

    for w_exp in range(min_exp, max_exp + 1):
        width = 1 << w_exp
        height = next_power_of_two(math.ceil(pixel_count / width))

        if not is_power_of_two(height):
            continue

        area = width * height
        if area < pixel_count:
            continue

        # Prefer close-to-square first, then less wasted area.
        squareness = abs(math.log2(width) - math.log2(height))
        waste = area - pixel_count

        candidate = (squareness, waste, max(width, height), width, height)

        if best is None or candidate < best:
            best = candidate

    if best is None:
        raise RuntimeError("Failed to find valid power-of-two dimensions")

    _, _, _, width, height = best
    return width, height


def repack_image(input_path: Path, output_path: Path) -> None:
    img = Image.open(input_path).convert("RGBA")
    src_w, src_h = img.size

    # Flatten the source into a single pixel stream in row-major order.
    pixels = list(img.getdata())
    pixel_count = len(pixels)

    if pixel_count == 0:
        raise ValueError("Input image has no pixels")

    out_w, out_h = choose_pow2_dimensions(pixel_count)

    # Transparent background for unused pixels.
    out = Image.new("RGBA", (out_w, out_h), (0, 0, 0, 0))
    out.putdata(
        pixels + [(0, 0, 0, 0)] * (out_w * out_h - pixel_count)
    )

    out.save(output_path)

    print(f"Input : {input_path} ({src_w}x{src_h}, {pixel_count} pixels)")
    print(f"Output: {output_path} ({out_w}x{out_h})")
    print(f"Unused output pixels: {out_w * out_h - pixel_count}")


def main() -> int:
    if len(sys.argv) != 3:
        print("Usage: python repack_png_pow2.py input.png output.png")
        return 1

    input_path = Path(sys.argv[1])
    output_path = Path(sys.argv[2])

    if not input_path.is_file():
        print(f"Error: input file not found: {input_path}")
        return 1

    try:
        repack_image(input_path, output_path)
    except Exception as exc:
        print(f"Error: {exc}")
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())