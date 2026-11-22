"""Generate the shared Nord app icon and macOS icon bundle."""

from pathlib import Path
import struct
import subprocess
import zlib


ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "src-tauri" / "icons"
SIZE = 1024
TRAY_SIZE = 64
BLUE = (0x81, 0xA1, 0xC1)
INK = (0x2E, 0x34, 0x40)


def inside_round_rect(x: float, y: float) -> bool:
    radius = 220
    cx = max(radius, min(SIZE - radius, x))
    cy = max(radius, min(SIZE - radius, y))
    return (x - cx) ** 2 + (y - cy) ** 2 <= radius**2


def pixel(x: int, y: int) -> bytes:
    samples = []
    for dy in (0.25, 0.75):
        for dx in (0.25, 0.75):
            sx, sy = x + dx, y + dy
            if not inside_round_rect(sx, sy):
                samples.append((0, 0, 0, 0))
                continue
            diamond = abs(sx - SIZE / 2) + abs(sy - SIZE / 2)
            color = INK if diamond <= 127 or 239 <= diamond <= 317 else BLUE
            samples.append((*color, 255))
    return bytes(round(sum(sample[channel] for sample in samples) / 4) for channel in range(4))


def chunk(kind: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def make_master() -> Path:
    rows = (b"\0" + b"".join(pixel(x, y) for x in range(SIZE)) for y in range(SIZE))
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(b"".join(rows), 9))
    png += chunk(b"IEND", b"")
    path = ICONS / "1024x1024.png"
    path.write_bytes(png)
    return path


def make_tray() -> Path:
    def tray_pixel(x: int, y: int) -> bytes:
        coverage = 0
        for dy in (0.25, 0.75):
            for dx in (0.25, 0.75):
                sx = (x + dx) * SIZE / TRAY_SIZE
                sy = (y + dy) * SIZE / TRAY_SIZE
                diamond = abs(sx - SIZE / 2) + abs(sy - SIZE / 2)
                coverage += diamond <= 127 or 239 <= diamond <= 317
        return bytes((0, 0, 0, round(coverage * 255 / 4)))

    rows = (b"\0" + b"".join(tray_pixel(x, y) for x in range(TRAY_SIZE)) for y in range(TRAY_SIZE))
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", TRAY_SIZE, TRAY_SIZE, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(b"".join(rows), 9))
    png += chunk(b"IEND", b"")
    path = ICONS / "tray.png"
    path.write_bytes(png)
    return path


def main() -> None:
    master = make_master()
    make_tray()
    subprocess.run(
        ["npm", "run", "tauri", "icon", str(master)],
        check=True,
        cwd=ROOT,
    )


if __name__ == "__main__":
    main()
