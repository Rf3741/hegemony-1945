#!/usr/bin/env python3
"""Gera a abertura litográfica do jogo sem usar imagens externas."""
from pathlib import Path
import math
import shutil
import subprocess
import tempfile
import wave

import numpy as np
from PIL import Image, ImageDraw, ImageFont

W, H, FPS, N = 1280, 720, 24, 240
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets/abertura"
FRAMES = OUT / "quadros"
FONT = ROOT / "assets/fonts/Oswald-Bold.ttf"
CREME, VERMELHO, AZUL, TINTA, AMARELO = "#efe4cc", "#b3261e", "#1f3a5f", "#171515", "#e8b530"


def font(size):
    return ImageFont.truetype(str(FONT), size)


def ease(t):
    t = max(0.0, min(1.0, t))
    return 1.0 - (1.0 - t) ** 3


def estrela(draw, cx, cy, raio, cor):
    pts = []
    for i in range(10):
        a = -math.pi / 2 + i * math.pi / 5
        r = raio if i % 2 == 0 else raio * 0.4
        pts.append((cx + math.cos(a) * r, cy + math.sin(a) * r))
    draw.polygon(pts, fill=cor)


def bezier(p0, p1, p2, p3, steps=24):
    pts = []
    for i in range(steps + 1):
        t = i / steps
        u = 1 - t
        pts.append((u**3*p0[0] + 3*u*u*t*p1[0] + 3*u*t*t*p2[0] + t**3*p3[0],
                    u**3*p0[1] + 3*u*u*t*p1[1] + 3*u*t*t*p2[1] + t**3*p3[1]))
    return pts


def linha_curva(draw, segments, fill, width=1, joint="curve"):
    pts = []
    for segment in segments:
        pts.extend(bezier(*segment))
    draw.line(pts, fill=fill, width=width, joint=joint)


def sombra(draw, shape, offset=(10, 10), fill=TINTA):
    draw.polygon([(x + offset[0], y + offset[1]) for x, y in shape], fill=fill)
    draw.polygon(shape, fill=fill)


def foice_martelo(draw, cx, cy, scale=1.0):
    gold = AMARELO
    # Sombra serigráfica, deliberadamente deslocada.
    draw.arc((cx-73*scale+8, cy-56*scale+9, cx+45*scale+8, cy+59*scale+9),
             205, 48, fill=TINTA, width=max(4, int(13*scale)))
    draw.line((cx-8*scale+8, cy+45*scale+9, cx+28*scale+8, cy-4*scale+9),
              fill=TINTA, width=max(4, int(11*scale)))
    draw.rectangle((cx+8*scale+8, cy-39*scale+9, cx+62*scale+8, cy-24*scale+9), fill=TINTA)
    # Lâmina crescente e cabo curto.
    draw.arc((cx-73*scale, cy-56*scale, cx+45*scale, cy+59*scale),
             205, 48, fill=gold, width=max(5, int(13*scale)))
    draw.line((cx-8*scale, cy+45*scale, cx+28*scale, cy-4*scale),
              fill=gold, width=max(5, int(11*scale)))
    # Martelo: cabo diagonal e cabeça retangular.
    draw.line((cx+7*scale, cy+40*scale, cx+39*scale, cy-32*scale),
              fill=gold, width=max(5, int(11*scale)))
    draw.rectangle((cx+18*scale, cy-45*scale, cx+73*scale, cy-27*scale), fill=gold)


def perfil(draw, side, color, p=1.0):
    left = side < 0
    # Contorno Bézier com testa, nariz, lábios, queixo e pescoço.
    if left:
        segs = [
            ((120, 525), (105, 430), (115, 300), (155, 230)),
            ((155, 230), (190, 185), (270, 180), (305, 220)),
            ((305, 220), (335, 245), (328, 270), (320, 291)),
            ((320, 291), (337, 301), (349, 311), (325, 320)),
            ((325, 320), (344, 332), (340, 344), (315, 349)),
            ((315, 349), (330, 375), (318, 410), (285, 430)),
            ((285, 430), (260, 451), (270, 510), (300, 555)),
            ((300, 555), (230, 575), (160, 570), (120, 525)),
        ]
        hat = [(135, 235), (150, 178), (214, 154), (285, 181), (305, 215), (170, 205)]
    else:
        segs = [
            ((1160, 525), (1175, 430), (1165, 300), (1125, 230)),
            ((1125, 230), (1090, 185), (1010, 180), (975, 220)),
            ((975, 220), (945, 245), (952, 270), (960, 291)),
            ((960, 291), (943, 301), (931, 311), (955, 320)),
            ((955, 320), (936, 332), (940, 344), (965, 349)),
            ((965, 349), (950, 375), (962, 410), (995, 430)),
            ((995, 430), (1020, 451), (1010, 510), (980, 555)),
            ((980, 555), (1050, 575), (1120, 570), (1160, 525)),
        ]
        hat = [(1145, 235), (1130, 178), (1066, 154), (995, 181), (975, 215), (1110, 205)]
    pts = []
    for seg in segs:
        pts.extend(bezier(*seg))
    scaled = [(640 + (x-640)*p, 360 + (y-360)*p) for x, y in pts]
    dshadow = [(x+9, y+10) for x, y in scaled]
    draw.polygon(dshadow, fill=TINTA)
    draw.polygon(scaled, fill=color, outline=CREME)
    hscaled = [(640 + (x-640)*p, 360 + (y-360)*p) for x, y in hat]
    draw.polygon([(x+8, y+9) for x, y in hscaled], fill=TINTA)
    draw.polygon(hscaled, fill=color, outline=CREME)
    draw.line((hscaled[0], hscaled[-1]), fill=CREME, width=3)


def globo(draw, gx, gy, gr, t):
    draw.ellipse((gx-gr+9, gy-gr+11, gx+gr+9, gy+gr+11), fill=TINTA)
    draw.ellipse((gx-gr, gy-gr, gx+gr, gy+gr), fill=CREME, outline=TINTA, width=7)
    # Paralelos e meridianos visíveis.
    for ratio in (-.62, -.32, 0, .32, .62):
        yy = gy + ratio * gr
        half = gr * math.sqrt(max(.05, 1-ratio*ratio))
        draw.arc((gx-half, yy-gr*.18, gx+half, yy+gr*.18), 180, 360, fill=AZUL, width=4)
        draw.arc((gx-half, yy-gr*.18, gx+half, yy+gr*.18), 0, 180, fill=AZUL, width=4)
    for ratio in (-.58, -.28, .28, .58):
        half = gr * (1-abs(ratio))*.72
        draw.ellipse((gx-half, gy-gr, gx+half, gy+gr), outline=AZUL, width=4)
    # Continentes estilizados.
    europe = [(gx-22, gy-55), (gx+5, gy-69), (gx+25, gy-52), (gx+18, gy-31),
              (gx+43, gy-20), (gx+20, gy-4), (gx-7, gy-14), (gx-30, gy-34)]
    africa = [(gx-4, gy-8), (gx+26, gy-2), (gx+16, gy+28), (gx+2, gy+60),
              (gx-18, gy+28)]
    americas = [(gx-75, gy-45), (gx-50, gy-60), (gx-48, gy-20), (gx-63, gy+5),
                (gx-45, gy+35), (gx-59, gy+72), (gx-77, gy+38)]
    for land in (europe, africa, americas):
        draw.polygon(land, fill=TINTA)
    # Arame serrilhado da Cortina de Ferro atravessando a Europa.
    wire = [(gx-74, gy-36), (gx-56, gy-48), (gx-42, gy-30), (gx-26, gy-47),
            (gx-10, gy-29), (gx+7, gy-45), (gx+23, gy-28), (gx+42, gy-42),
            (gx+68, gy-25)]
    draw.line(wire, fill=TINTA, width=11, joint="curve")
    for x, y in wire[1:-1]:
        draw.line((x-8, y-11, x+8, y+11), fill=TINTA, width=3)


def missil(draw, side, q):
    x = 100 if side < 0 else 1180
    y = 735 - 350*q
    direction = 1 if side < 0 else -1
    # Rastro curvo e chama animada.
    smoke = [(x-direction*18, y+70), (x-direction*75, y+115),
             (x-direction*145, y+92), (x-direction*205, y+145)]
    draw.line(smoke, fill=CREME, width=34, joint="curve")
    draw.line(smoke, fill=TINTA, width=9, joint="curve")
    flame = [(x, y+60), (x-direction*18, y+105), (x, y+89),
             (x+direction*14, y+112)]
    draw.polygon(flame, fill="#e47725")
    body = [(x-17, y+55), (x-17, y-28), (x, y-53), (x+17, y-28), (x+17, y+55)]
    draw.polygon([(a+9, b+10) for a, b in body], fill=TINTA)
    draw.polygon(body, fill=CREME, outline=TINTA)
    draw.polygon([(x-17, y-28), (x, y-53), (x+17, y-28)], fill=VERMELHO)
    draw.rectangle((x-18, y-3, x+18, y+9), fill=VERMELHO)
    draw.polygon([(x-17, y+38), (x-42, y+62), (x-17, y+58)], fill=TINTA)
    draw.polygon([(x+17, y+38), (x+42, y+62), (x+17, y+58)], fill=TINTA)


def cartaz(t):
    rng = np.random.default_rng(9000 + int(t * FPS))
    im = Image.new("RGB", (W, H), CREME)
    d = ImageDraw.Draw(im)
    # Sunburst lento e a divisão diagonal.
    ang0 = t * 0.18
    for i in range(24):
        a0, a1 = ang0 + i * math.pi / 12, ang0 + (i + .52) * math.pi / 12
        d.polygon([(640, 350), (640 + 1100 * math.cos(a0), 350 + 1100 * math.sin(a0)),
                   (640 + 1100 * math.cos(a1), 350 + 1100 * math.sin(a1))],
                  fill=VERMELHO if i % 2 else CREME)
    if t >= 1:
        p = ease((t - 1) / 2)
        d.polygon([(0, 0), (W * (0.56 * p), 0), (W * (0.18 * p), H), (0, H)], fill=AZUL)
        d.polygon([(W, 0), (W, H), (W * (0.65 + .35 * p), H), (W * (0.46 + .25 * p), 0)],
                  fill=VERMELHO)
        estrela(d, 210, 205, 92 * p, CREME)
        for y in range(390, 690, 54):
            d.line((0, y, 520, y - 100), fill=CREME, width=18)
        estrela(d, 1080, 200, 68 * p, AMARELO)
        foice_martelo(d, 1080, 335, p*.78)
    # Perfis de silhueta e globo.
    if t >= 3:
        p = ease((t - 3) / 2)
        perfil(d, -1, AZUL, p)
        perfil(d, 1, VERMELHO, p)
        globo(d, 640, 350, 202*p, t)
    # Título, mísseis e selo.
    if t >= 5:
        for i, ch in enumerate("HEGEMONY"):
            q = ease((t - 5 - i * .12) / .55)
            x = 235 + i * 116
            y = 82 - 35 * (1 - q) + abs(math.sin(q * math.pi)) * 7
            d.text((x + 8, y + 10), ch, font=font(112), fill=TINTA, anchor="mm")
            d.text((x, y), ch, font=font(112), fill=CREME, stroke_width=2, stroke_fill=TINTA, anchor="mm")
        d.rectangle((455, 485, 825, 590), fill=TINTA)
        d.rectangle((445, 475, 815, 580), fill=VERMELHO)
        d.text((630, 527), "1945", font=font(106), fill=CREME, anchor="mm")
    if t >= 7:
        q = ease((t - 7) / 1.4)
        missil(d, -1, q)
        missil(d, 1, q)
        d.rectangle((190, 631, 1090, 674), fill=(23, 21, 21, 170))
        d.text((640, 652), "1º DE JANEIRO DE 1945 — O MUNDO SE DIVIDE",
                font=font(31), fill=CREME, anchor="mm")
    if t >= 8.5:
        q = min(1.0, (t - 8.5) / .3)
        scale = 1.4 - .4*q
        stamp = Image.new("RGBA", (760, 250), (0, 0, 0, 0))
        sd = ImageDraw.Draw(stamp)
        box = (35, 40, 725, 210)
        sd.rounded_rectangle((box[0]+10, box[1]+11, box[2]+10, box[3]+11), radius=10,
                             outline=TINTA, width=8)
        sd.rounded_rectangle(box, radius=10, outline=VERMELHO, width=8)
        sd.rounded_rectangle((box[0]+17, box[1]+17, box[2]-17, box[3]-17), radius=5,
                             outline=VERMELHO, width=5)
        sd.text((380, 125), "TOP SECRET", font=font(82), fill=VERMELHO, anchor="mm")
        for x, y in ((100, 80), (250, 177), (435, 91), (620, 160), (555, 185)):
            sd.rectangle((x, y, x+35, y+7), fill=(239, 228, 204, 150))
        stamp = stamp.resize((int(760*scale), int(250*scale)), Image.Resampling.BICUBIC)
        stamp = stamp.rotate(12, resample=Image.Resampling.BICUBIC, expand=True)
        im.paste(stamp, (int(640-stamp.width/2), int(370-stamp.height/2)), stamp)
    # Papel, granulação e vinheta.
    arr = np.asarray(im, dtype=np.int16)
    arr += rng.normal(0, 3.2, (H, W, 1)).astype(np.int16)
    yy, xx = np.ogrid[:H, :W]
    vignette = ((xx - W/2) ** 2 / (W/2) ** 2 + (yy - H/2) ** 2 / (H/2) ** 2)
    arr -= np.clip((vignette - .55) * 7, 0, 12).astype(np.int16)[..., None]
    return Image.fromarray(np.uint8(np.clip(arr, 0, 255)))


def audio(path):
    sr, dur = 44100, 10.2
    t = np.arange(int(sr * dur)) / sr
    rng = np.random.default_rng(1945)
    y = rng.normal(0, .018, t.size)
    y += .012 * np.sin(2 * np.pi * 92 * t)

    def add(start, end, fn, gain):
        a, b = int(start * sr), min(len(y), int(end * sr))
        y[a:b] += gain * fn(np.arange(b - a) / sr)

    add(4.5, 6.9, lambda x: np.sin(2*np.pi*5*x) * np.exp(-x*.15), .22)
    for f in (196, 247, 294):
        add(6.0, 7.4, lambda x, f=f: np.sin(2*np.pi*f*x) + .25*np.sin(2*np.pi*f*2*x), .10)
    add(7.0, 8.4, lambda x: np.sin(2*np.pi*(900 - 700*x)*x) * np.exp(-x*.4), .16)
    add(8.5, 8.9, lambda x: np.sin(2*np.pi*65*x) * np.exp(-x*16), .65)
    y *= np.minimum(1, np.maximum(0, (dur - t) / .8))
    y /= max(1.0, np.max(np.abs(y)) * 1.02)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(sr)
        w.writeframes((y * 32767).astype("<i2").tobytes())


def main():
    FRAMES.mkdir(parents=True, exist_ok=True)
    for i in range(N):
        cartaz(i / FPS).save(FRAMES / f"{i:04}.jpg", quality=70, optimize=True)
    with tempfile.TemporaryDirectory() as tmp:
        wav = Path(tmp) / "abertura.wav"
        audio(wav)
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(wav), "-c:a", "libvorbis", "-q:a", "5",
                        str(OUT / "abertura.ogg")], check=True)
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-framerate", str(FPS), "-i",
                    str(FRAMES / "%04d.jpg"), "-i", str(OUT / "abertura.ogg"), "-c:v", "libx264",
                    "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest", str(Path(__file__).with_name("previa.mp4"))],
                   check=True)
    folha = Image.new("RGB", (1200, 900), CREME)
    for n, i in enumerate(np.linspace(0, N-1, 12).astype(int)):
        thumb = Image.open(FRAMES / f"{i:04}.jpg").resize((300, 169))
        folha.paste(thumb, ((n % 4) * 300, (n // 4) * 300))
    folha.save(Path(__file__).with_name("folha_contato.png"))
    print(f"{N} quadros, {sum(p.stat().st_size for p in FRAMES.glob('*.jpg')) / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
