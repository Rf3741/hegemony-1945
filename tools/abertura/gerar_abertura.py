#!/usr/bin/env python3
"""Gera a abertura litográfica do jogo sem usar imagens externas."""
from pathlib import Path
import math
import shutil
import subprocess
import tempfile
import wave

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFont

W, H, FPS, N = 1280, 720, 24, 240
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets/abertura"
FRAMES = OUT / "quadros"
FONT = ROOT / "assets/fonts/Oswald-Bold.ttf"
CREME, VERMELHO, AZUL, TINTA, AMARELO = "#efe4cc", "#b3261e", "#1f3a5f", "#171515", "#e8b530"
CONTINENTE = (159, 179, 200, 255)
CONTORNO_TERRA = (58, 74, 90, 255)
GRADE = "#5f7fa3"
CARIMBO = "#c02a26"


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


def suavizar(pontos, fechado=True, steps=14):
    """Catmull-Rom -> malha densa, para contornos contínuos sem cantos vivos."""
    n = len(pontos)
    saida = []
    faixa = range(n) if fechado else range(n - 1)
    for i in faixa:
        if fechado:
            p0, p1, p2, p3 = (pontos[(i - 1) % n], pontos[i],
                              pontos[(i + 1) % n], pontos[(i + 2) % n])
        else:
            p0 = pontos[max(i - 1, 0)]
            p1, p2 = pontos[i], pontos[i + 1]
            p3 = pontos[min(i + 2, n - 1)]
        for k in range(steps):
            t = k / steps
            t2, t3 = t * t, t * t * t
            saida.append(tuple(
                0.5 * (2 * p1[d] + (-p0[d] + p2[d]) * t
                       + (2 * p0[d] - 5 * p1[d] + 4 * p2[d] - p3[d]) * t2
                       + (-p0[d] + 3 * p1[d] - 3 * p2[d] + p3[d]) * t3)
                for d in (0, 1)))
    return saida


def bloco(draw, pts, cor, contorno=TINTA, esp=6):
    """Forma preenchida com contorno escuro por fora."""
    draw.line(pts + [pts[0]], fill=contorno, width=esp, joint="curve")
    draw.polygon(pts, fill=cor)


def foice_martelo(draw, cx, cy, h):
    """Emblema desenhado em unidades próprias (~152u de altura)."""
    u = h / 140.0

    def P(x, y):
        return (cx + x * u, cy + y * u)

    # Foice: crescente de ~270° aberta para a direita, afinando até a ponta.
    mx, my, raio = -20.0, -20.0, 50.0
    externa, interna = [], []
    for i in range(61):
        k = i / 60
        g = math.radians(45 + 270 * k)
        meia = 10.5 - 8.3 * k ** 1.5
        x, y = mx + raio * math.cos(g), my + raio * math.sin(g)
        externa.append(P(x + math.cos(g) * meia, y + math.sin(g) * meia))
        interna.append(P(x - math.cos(g) * meia, y - math.sin(g) * meia))
    lamina = externa + interna[::-1]
    cabo_foice = [P(21, 9), P(36, 24), P(52, 47), P(58, 61),
                  P(46, 68), P(39, 55), P(26, 37), P(11, 23)]
    # Martelo: cabo reto na diagonal oposta (cruzando a foice em X) e cabeça grossa.
    cabo_martelo = [P(-58, 78), P(-46, 62), P(40, -26), P(52, -38),
                    P(40, -48), P(-48, 42), P(-64, 70)]
    cabeca = [P(4, -74), P(72, -66), P(68, -34), P(0, -42)]

    pecas = [lamina, cabo_foice, cabo_martelo, cabeca]
    for pts in pecas:
        draw.polygon([(x + 7 * u, y + 8 * u) for x, y in pts], fill=TINTA)
    for pts in pecas:
        bloco(draw, pts, AMARELO, esp=max(3, int(4 * u)))


PERFIL_CONTORNO = [
    (95, 8), (140, 22), (156, 54), (148, 68), (158, 80), (182, 98),
    (152, 108), (164, 120), (140, 130), (158, 142), (146, 154), (168, 172),
    (124, 196), (142, 232), (162, 274), (208, 314), (252, 402), (-72, 402),
    (-16, 302), (18, 252), (32, 210), (12, 150), (-2, 96), (22, 40),
]
PERFIL_QUEPE = [(-8, 44), (-12, 14), (16, -14), (88, -26), (142, -12),
                (166, 24), (150, 40), (2, 46)]
PERFIL_ABA = [(144, 32), (198, 42), (202, 58), (150, 52)]


def perfil(draw, side, color, p=1.0):
    """Busto em perfil; cabeça entre 8 e 196 nas unidades locais."""
    s = 1.55 * p
    f = 1 if side < 0 else -1          # o perfil da esquerda olha para a direita
    ox = 345 if side < 0 else 935
    oy = 372
    ax, ay = 90.0, 110.0

    def T(x, y):
        return (ox + f * s * (x - ax), oy + s * (y - ay))

    pts = [T(*q) for q in suavizar(PERFIL_CONTORNO, True, 12)]
    draw.polygon([(x + 9, y + 10) for x, y in pts], fill=TINTA)
    bloco(draw, pts, color, contorno=CREME, esp=6)

    qpts = [T(*q) for q in suavizar(PERFIL_QUEPE, True, 8)]
    draw.polygon([(x + 8, y + 9) for x, y in qpts], fill=TINTA)
    bloco(draw, qpts, color, contorno=CREME, esp=5)
    aba = [T(*q) for q in suavizar(PERFIL_ABA, True, 6)]
    draw.polygon([(x + 6, y + 7) for x, y in aba], fill=TINTA)
    bloco(draw, aba, TINTA, contorno=CREME, esp=4)


CONTINENTES = [
    # Europa, África, Ásia e as duas Américas em coordenadas do disco (-100..100).
    [(-30, -46), (-38, -58), (-26, -64), (-16, -56), (-8, -72), (6, -76),
     (16, -62), (28, -56), (32, -44), (20, -36), (4, -30), (-10, -34), (-22, -40)],
    [(-24, -24), (-4, -28), (18, -30), (34, -22), (40, -6), (32, 12),
     (24, 34), (10, 52), (-4, 40), (-14, 18), (-26, 4), (-18, -10)],
    [(28, -72), (50, -82), (76, -72), (94, -54), (98, -34), (84, -24),
     (72, -10), (60, -18), (52, 6), (44, -8), (36, -22), (30, -40), (24, -56)],
    [(-98, -56), (-80, -70), (-58, -64), (-48, -46), (-56, -34),
     (-70, -28), (-82, -16), (-92, -26), (-98, -42)],
    [(-76, -12), (-58, -8), (-50, 10), (-58, 30), (-70, 48),
     (-82, 40), (-86, 18), (-80, 2)],
]


def globo(im, gx, gy, gr):
    if gr < 10:
        return
    d = ImageDraw.Draw(im)
    r = gr
    d.ellipse((gx - r + 9, gy - r + 11, gx + r + 9, gy + r + 11), fill=TINTA)
    d.ellipse((gx - r, gy - r, gx + r, gy + r), fill=CREME)

    lado = int(2 * r) + 6
    c = lado / 2.0
    U = r / 100.0
    tile = Image.new("RGBA", (lado, lado), (0, 0, 0, 0))
    td = ImageDraw.Draw(tile)

    def P(x, y):
        return (c + x * U, c + y * U)

    # Grade discreta, para não brigar com as terras.
    for lat in (-62, -34, 0, 34, 62):
        meia = math.sqrt(max(0.0, 10000 - lat * lat))
        td.ellipse((c - meia * U, c + lat * U - meia * U * .2,
                    c + meia * U, c + lat * U + meia * U * .2), outline=GRADE, width=3)
    for fra in (0.34, 0.68, 1.0):
        td.ellipse((c - fra * 100 * U, c - 100 * U, c + fra * 100 * U, c + 100 * U),
                   outline=GRADE, width=3)

    # Terras em azul-acinzentado claro, com contorno fino.
    for terra in CONTINENTES:
        pts = [P(*q) for q in suavizar(terra, True, 8)]
        td.line(pts + [pts[0]], fill=CONTORNO_TERRA, width=max(2, int(3 * U)), joint="curve")
        td.polygon(pts, fill=CONTINENTE)

    # Cortina de Ferro: serra preta grossa apenas sobre a Europa.
    dentes = []
    n = 11
    for i in range(n + 1):
        x = -44 + 80 * i / n
        y = -50 + (8 if i % 2 else -8) + 6 * i / n
        dentes.append(P(x, y))
    td.line(dentes, fill=TINTA, width=max(4, int(6 * U)), joint="curve")
    for (x, y) in dentes[1:-1]:
        e = 7 * U
        td.line((x - e, y - e, x + e, y + e), fill=TINTA, width=max(2, int(3 * U)))
        td.line((x - e, y + e, x + e, y - e), fill=TINTA, width=max(2, int(3 * U)))

    mascara = Image.new("L", (lado, lado), 0)
    ImageDraw.Draw(mascara).ellipse((3, 3, lado - 4, lado - 4), fill=255)
    tile.putalpha(ImageChops.multiply(tile.getchannel("A"), mascara))
    im.paste(tile, (int(gx - r - 3), int(gy - r - 3)), tile)
    d.ellipse((gx - r, gy - r, gx + r, gy + r), outline=TINTA, width=7)


def missil(draw, side, q):
    if q <= 0:
        return
    dir_ = 1 if side < 0 else -1
    xb = 70 if side < 0 else 1210
    trilha = bezier((xb, 824), (xb + dir_ * 12, 700),
                    (xb + dir_ * 46, 592), (xb + dir_ * 70, 478), steps=48)
    i = max(1, min(48, int(q * 48)))
    trecho = trilha[:i + 1]
    atual, anterior = trilha[i], trilha[max(i - 1, 0)]
    ang = math.atan2(atual[1] - anterior[1], atual[0] - anterior[0]) + math.pi / 2

    # Nuvem de fumaça: volumosa na base e afinando atrás do míssil.
    nuvens = []
    for k in range(0, i + 1, 4):
        f = k / max(i, 1)
        raio = int(30 - 16 * f)
        cx, cy = trecho[k]
        nuvens.append((cx, cy, raio))
    for cx, cy, raio in nuvens:
        draw.ellipse((cx - raio - 5, cy - raio - 5, cx + raio + 5, cy + raio + 5), fill=TINTA)
    draw.line(trecho, fill=TINTA, width=48, joint="curve")
    for cx, cy, raio in nuvens:
        draw.ellipse((cx - raio, cy - raio, cx + raio, cy + raio), fill=CREME)
    draw.line(trecho, fill=CREME, width=38, joint="curve")

    s = 1.25
    ca, sa = math.cos(ang), math.sin(ang)

    def M(x, y):
        x, y = x * s, y * s
        return (atual[0] + x * ca - y * sa, atual[1] + x * sa + y * ca)

    corpo = [(-20, 36), (-20, -38), (0, -72), (20, -38), (20, 36)]
    cone = [(-20, -38), (0, -72), (20, -38)]
    faixa = [(-20, -8), (20, -8), (20, 10), (-20, 10)]
    aletas = [[(-20, 6), (-48, 46), (-20, 42)], [(20, 6), (48, 46), (20, 42)]]
    chama = [(-13, 34), (0, 74 + 10 * math.sin(q * 20)), (13, 34)]

    for peca in [corpo] + aletas:
        draw.polygon([(x + 9, y + 10) for x, y in (M(*p) for p in peca)], fill=TINTA)
    for peca in aletas:
        draw.polygon([M(*p) for p in peca], fill=TINTA)
    bloco(draw, [M(*p) for p in corpo], CREME, esp=5)
    draw.polygon([M(*p) for p in cone], fill=VERMELHO)
    bloco(draw, [M(*p) for p in faixa], VERMELHO, esp=4)
    draw.polygon([M(*p) for p in chama], fill="#e47725")
    draw.polygon([M(*p) for p in [(-6, 34), (0, 56), (6, 34)]], fill=AMARELO)


def carimbo(im, cx, cy, w, h, q):
    """Carimbo grande e gasto, com lavagem clara para ler sobre qualquer fundo."""
    pad = 34
    cw, ch = w + 2 * pad, h + 2 * pad
    stamp = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
    sd = ImageDraw.Draw(stamp)
    sd.rounded_rectangle((pad - 16, pad - 16, cw - pad + 16, ch - pad + 16), radius=18,
                         fill=(239, 228, 204, 125))
    sd.rounded_rectangle((pad + 10, pad + 11, cw - pad + 10, ch - pad + 11), radius=12,
                         outline=(23, 21, 21, 110), width=9)
    bx = (pad, pad, cw - pad, ch - pad)
    sd.rounded_rectangle(bx, radius=12, outline=CARIMBO, width=12)
    sd.rounded_rectangle((bx[0] + 17, bx[1] + 17, bx[2] - 17, bx[3] - 17), radius=7,
                         outline=CARIMBO, width=5)
    tam = h - 30
    while tam > 20 and sd.textlength("TOP SECRET", font=font(tam)) > w - 80:
        tam -= 2
    sd.text((cw / 2, ch / 2 + 2), "TOP SECRET", font=font(tam), fill=CARIMBO,
            anchor="mm", stroke_width=4, stroke_fill=CARIMBO)

    gasto = Image.new("L", (cw, ch), 255)
    gd = ImageDraw.Draw(gasto)
    rng = np.random.default_rng(7719)
    for _ in range(520):
        x, y = int(rng.integers(0, cw)), int(rng.integers(0, ch))
        r = int(rng.integers(1, 5))
        gd.ellipse((x - r, y - r, x + r, y + r), fill=int(rng.integers(40, 200)))
    for _ in range(18):
        y = int(rng.integers(pad - 10, ch - pad + 10))
        gd.line((pad - 14, y, cw - pad + 14, y + int(rng.integers(-12, 12))),
                fill=int(rng.integers(30, 130)), width=int(rng.integers(2, 8)))
    stamp.putalpha(ImageChops.multiply(stamp.getchannel("A"), gasto))

    escala = 1.30 - 0.30 * q
    if abs(escala - 1.0) > 0.01:
        stamp = stamp.resize((max(1, int(cw * escala)), max(1, int(ch * escala))),
                             Image.Resampling.BICUBIC)
    stamp = stamp.rotate(12, resample=Image.Resampling.BICUBIC, expand=True)
    im.paste(stamp, (int(cx - stamp.width / 2), int(cy - stamp.height / 2)), stamp)


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
        estrela(d, 112, 138, 60 * p, CREME)
        for y in range(390, 690, 54):
            d.line((0, y, 520, y - 100), fill=CREME, width=18)
        estrela(d, 1185, 70, 36 * p, AMARELO)
        foice_martelo(d, 1195, 192, 132 * p)
    # Perfis de silhueta e globo.
    if t >= 3:
        p = ease((t - 3) / 2)
        perfil(d, -1, AZUL, p)
        perfil(d, 1, VERMELHO, p)
        globo(im, 640, 272, 138 * p)
    # Título, mísseis e selo.
    if t >= 5:
        for i, ch in enumerate("HEGEMONY"):
            q = ease((t - 5 - i * .12) / .55)
            x = 247 + i * 112
            y = 76 - 35 * (1 - q) + abs(math.sin(q * math.pi)) * 7
            d.text((x + 8, y + 10), ch, font=font(106), fill=TINTA, anchor="mm")
            d.text((x, y), ch, font=font(106), fill=CREME, stroke_width=2, stroke_fill=TINTA, anchor="mm")
        d.rectangle((510, 378, 790, 462), fill=TINTA)
        d.rectangle((500, 368, 780, 452), fill=VERMELHO)
        d.text((640, 410), "1945", font=font(82), fill=CREME, anchor="mm")
    if t >= 7:
        q = ease((t - 7) / 1.4)
        missil(d, -1, q)
        missil(d, 1, q)
        d.rectangle((170, 690, 1110, 720), fill=(23, 21, 21, 235))
        d.text((640, 705), "1º DE JANEIRO DE 1945 — O MUNDO SE DIVIDE",
                font=font(26), fill=CREME, anchor="mm")
    if t >= 8.5:
        q = min(1.0, (t - 8.5) / .3)
        carimbo(im, 640, 562, 576, 104, q)
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
