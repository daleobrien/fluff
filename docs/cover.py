# Cover for fluff: the real 13-byte repeat.br, adrift in the 1 MiB of 'a' it expands to.
import sys
W, H = 1280, 640
fs, cw, lh = 20, 12.0, 24          # 'a' wall: font size, advance, line height
bytes_ = "5f ff ff 0f 00 22 2c 1e 0b 28 f7 fe 01".split()
out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">',
       f'<rect x="1" y="1" width="{W-2}" height="{H-2}" rx="18" fill="#fff" stroke="#ddd" stroke-width="2"/>',
       '<g font-family="DejaVu Sans Mono, Menlo, monospace" fill="#111">']
x0, y0 = 44, 52
cols = int((W - 2 * x0) // cw)
x0 = (W - cols * cw) / 2
rows = int((H - y0 - 30) // lh) + 1
line = "a" * cols
rows = int((H - y0 - 24) // lh) + 1
y0 = (H - (rows - 1) * lh) / 2 + 7          # centre the wall (+7: baseline vs x-height)
bw, bh = 560, 190
bx, by = (W - bw) / 2, (H - bh) / 2
gap = (int((W - bw) / 2 - x0 - 18) // int(cw))   # columns kept left and right of the card
for r in range(rows):
    y = y0 + r * lh
    if by - 14 < y < by + bh + 30:          # rows the card sits on: leave a gap
        for xs, n in ((x0, gap), (W - x0 - gap * cw, gap)):
            out.append(f'<text x="{xs:.0f}" y="{y:.0f}" font-size="{fs}" textLength="{n*cw:.0f}">{"a"*n}</text>')
    else:
        out.append(f'<text x="{x0}" y="{y:.0f}" font-size="{fs}" textLength="{cols*cw:.0f}">{line}</text>')
out.append('</g>')
# the compressed file: a red-bordered card holding the real bytes
out.append(f'<rect x="{bx}" y="{by}" width="{bw}" height="{bh}" fill="#fff" stroke="#e01b24" stroke-width="5"/>')
rows_b = [bytes_[:7], bytes_[7:]]
for i, rb in enumerate(rows_b):
    out.append(f'<text x="{W/2}" y="{by + 82 + i*62}" text-anchor="middle" font-family="DejaVu Sans Mono, Menlo, monospace" '
               f'font-size="40" font-weight="bold" fill="#000">{" ".join(rb)}</text>')
out.append('</svg>')
open(sys.argv[1], "w").write("\n".join(out))
