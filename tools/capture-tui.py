# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow", "pyte"]
# ///
"""Capture the running TUI's actual PTY screen as a PNG.

Usage: uv run tools/capture-tui.py --binary target/debug/meden \
  --workspace /tmp/meden-demo --provider hermes:ollama-cloud \
  --model minimax-m3 --font /path/to/JetBrainsMono-Regular.otf

The image contains only cells emitted by the real application. It does not
invent model replies or tool results and makes no model inference request.
"""

import argparse
import codecs
import fcntl
import os
import pty
import select
import struct
import subprocess
import termios
import time
from pathlib import Path

import pyte
from PIL import Image, ImageDraw, ImageFont


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--provider", required=True)
    parser.add_argument("--model")
    parser.add_argument("--font", type=Path, required=True)
    parser.add_argument("--out", type=Path, default=Path("docs/assets/tui.png"))
    parser.add_argument(
        "--screen", choices=["picker", "instruction", "processor-approval"], default="instruction"
    )
    parser.add_argument("--columns", type=int, default=112)
    parser.add_argument("--rows", type=int, default=24)
    args = parser.parse_args()
    columns, rows = args.columns, args.rows
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
    initial = termios.tcgetattr(slave)
    environment = {**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor"}
    environment.pop("NO_COLOR", None)
    process = subprocess.Popen(
        [
            str(args.binary.resolve()),
            "tui",
            "--workspace",
            str(args.workspace),
            "--provider",
            args.provider,
            *(["--model", args.model] if args.model else []),
        ],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=environment,
    )
    screen = pyte.Screen(columns, rows)
    stream = pyte.Stream(screen)
    decoder = codecs.getincrementaldecoder("utf-8")("replace")
    raw = bytearray()

    def pump(predicate):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.1)[0]:
                data = os.read(master, 65536)
                raw.extend(data)
                stream.feed(decoder.decode(data))
            if predicate():
                return
            if process.poll() is not None:
                raise RuntimeError(raw.decode(errors="replace"))
        raise TimeoutError("TUI capture timed out")

    try:
        pump(lambda: "choose" in "\n".join(screen.display)
             or "Connection details" in "\n".join(screen.display))
        if args.screen == "picker" and "Connection details" in "\n".join(screen.display):
            os.write(master, b"n")
            pump(lambda: "choose" in "\n".join(screen.display))
        if args.screen != "picker" and "Connection details" not in "\n".join(screen.display):
            os.write(master, b"\r")
            pump(lambda: "Connection details" in "\n".join(screen.display))
        if args.screen == "instruction":
            os.write(master, b"y")
            pump(lambda: "alt-enter" in "\n".join(screen.display))
            prompt = "Inspect this project, improve the implementation, and run its tests."
            os.write(master, b"\x1b[200~" + prompt.encode() + b"\x1b[201~")
            pump(lambda: prompt in "\n".join(screen.display))
        # Finish receiving the frame before recording its actual cells.
        until = time.monotonic() + 0.5
        while time.monotonic() < until and select.select([master], [], [], 0.1)[0]:
            data = os.read(master, 65536)
            raw.extend(data)
            stream.feed(decoder.decode(data))
        font = ImageFont.truetype(str(args.font), 20)
        cell_width, cell_height, padding = round(font.getlength("M")), 27, 24
        default_bg, default_fg = "#0c121e", "#dce6ef"
        palette = {
            "black": "#152337",
            "red": "#f57989",
            "green": "#8ed6ac",
            "brown": "#e9c580",
            "blue": "#8bb6ee",
            "magenta": "#bb9ce9",
            "cyan": "#68dadb",
            "white": "#dce6ef",
            "default": default_fg,
        }

        def color(value, default):
            if value == "default":
                return default
            if value in palette:
                return palette[value]
            if len(value) == 6:
                return f"#{value}"
            return default

        image = Image.new(
            "RGB",
            (columns * cell_width + 2 * padding, rows * cell_height + 2 * padding),
            default_bg,
        )
        draw = ImageDraw.Draw(image)
        for y in range(rows):
            for x in range(columns):
                cell = screen.buffer[y][x]
                fg, bg = color(cell.fg, default_fg), color(cell.bg, default_bg)
                if cell.reverse:
                    fg, bg = bg, fg
                left, top = padding + x * cell_width, padding + y * cell_height
                draw.rectangle((left, top, left + cell_width - 1, top + cell_height - 1), fill=bg)
                if cell.data == "█":
                    draw.rectangle(
                        (left, top, left + cell_width - 1, top + cell_height - 1), fill=fg
                    )
                elif cell.data:
                    draw.text((left, top), cell.data, font=font, fill=fg,
                              stroke_width=1 if cell.bold else 0)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        image.save(args.out)
        args.out.with_suffix(".txt").write_text(
            "\n".join(line.rstrip() for line in screen.display).rstrip() + "\n"
        )
        os.write(master, b"\x03")
        process.wait(timeout=5)
        if process.returncode != 0 or termios.tcgetattr(slave) != initial:
            raise RuntimeError("TUI did not exit cleanly or restore the terminal")
        print(args.out)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    main()
