#!/usr/bin/env python3
"""Gera retratos estilizados (SVG -> PNG via rsvg-convert) para todos os personagens.

Saída: assets/retratos/{id}.png (128x160), mais retrato_generico.png.
Uso: python3 scripts/gerar_retratos.py
"""
import hashlib, os, re, subprocess, sys
RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DADOS = os.path.join(RAIZ, "assets/data/personagens")
SAIDA = os.path.join(RAIZ, "assets/retratos")

FUNDO = {  # ideologia -> (topo, base)
    "democracia_liberal": ("#3b6ea8", "#1f3d63"), "liberalismo": ("#4a86b8", "#25506f"),
    "social_democracia": ("#c0504d", "#7a2523"), "socialismo_democratico": ("#cf6a55", "#8a3326"),
    "democracia_crista": ("#d6a93a", "#8a6a15"), "conservadorismo": ("#5f6f86", "#323d4d"),
    "conservadorismo_autoritario": ("#6b5a4a", "#392e24"), "tradicionalismo_monarquico": ("#7a4b9a", "#3f2356"),
    "nacionalismo_republicano": ("#3e8e6a", "#1d4d38"), "nacionalismo_anticolonial": ("#4a9a54", "#25562c"),
    "militarismo_nacionalista": ("#6e7a3c", "#383f1b"), "comunismo_marxista_leninista": ("#b32424", "#600f0f"),
    "marxismo_leninismo": ("#b32424", "#600f0f"), "trotskismo": ("#8f1f2f", "#4a0c16"),
    "socialismo_arabe": ("#2f8f7f", "#144b42"), "islamismo_politico": ("#2e7d5b", "#124432"),
    "fascismo": ("#222222", "#080808"), "fascismo_residual": ("#2a2a2a", "#0c0c0c"),
    "colonialismo_imperial": ("#9a7b3c", "#54401a"),
}
PELE = ["#f1c9a5", "#e0ac84", "#c68a62", "#a56b45", "#7d4a2d", "#f4d6bd"]
CABELO = ["#1c1a18", "#3b2a1c", "#6b4a2b", "#9a9a9a", "#d8d8d8", "#5a3b22"]
GRAVATA = ["#8c1c1c", "#1c3a7a", "#2a2a2a", "#6a4b1a", "#1f5c3a"]

def h(s, n):
    return int(hashlib.md5((s + str(n)).encode()).hexdigest()[:8], 16)

def iniciais(nome):
    ps = [p for p in re.split(r"[\s.]+", nome) if p and p[0].isalpha() and p.lower() not in ("de", "da", "do", "von", "van", "bin", "al", "el")]
    if not ps:
        return "?"
    return (ps[0][0] + (ps[-1][0] if len(ps) > 1 else "")).upper()

def uniforme(p):
    return p["cargo_inicial"] == "comandante_militar" or p["ideologia"] in ("militarismo_nacionalista", "fascismo", "fascismo_residual") \
        or (p["ideologia"] in ("comunismo_marxista_leninista", "marxismo_leninismo") and h(p["id"], 9) % 2 == 0)

def svg(p):
    i = p["id"]
    topo, base = FUNDO.get(p["ideologia"], ("#707070", "#303030"))
    pele, cab = PELE[h(i, 1) % len(PELE)], CABELO[h(i, 2) % len(CABELO)]
    grav = GRAVATA[h(i, 3) % len(GRAVATA)]
    idade = 1945 - int(p["nascimento"][:4])
    if idade > 62 and h(i, 2) % 3:
        cab = CABELO[3 + h(i, 4) % 2]
    estilo = h(i, 5) % 4  # 0 curto, 1 lateral, 2 calvo, 3 volumoso
    mil = uniforme(p)
    cor_corpo = ("#4f5a3a" if h(i, 6) % 2 else "#59606b") if mil else ("#262a33" if h(i, 6) % 2 else "#3a3f4a")
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" width="128" height="160" viewBox="0 0 128 160">',
         f'<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{topo}"/><stop offset="1" stop-color="{base}"/></linearGradient></defs>',
         '<rect width="128" height="160" fill="url(#g)"/>',
         # corpo
         f'<path d="M10 160 L14 128 Q18 112 44 106 L84 106 Q110 112 114 128 L118 160 Z" fill="{cor_corpo}"/>',
         f'<rect x="54" y="92" width="20" height="18" rx="4" fill="{pele}"/>']
    if mil:
        o += ['<path d="M50 108 L64 128 L78 108 Z" fill="#cfc8b4"/>',
              f'<rect x="12" y="118" width="22" height="7" fill="#c9a13b"/><rect x="94" y="118" width="22" height="7" fill="#c9a13b"/>',
              f'<path d="M44 106 L64 132 L50 140 L36 118 Z" fill="{cor_corpo}" stroke="#00000055"/><path d="M84 106 L64 132 L78 140 L92 118 Z" fill="{cor_corpo}" stroke="#00000055"/>',
              '<circle cx="42" cy="142" r="3" fill="#c9a13b"/><circle cx="86" cy="142" r="3" fill="#c9a13b"/>']
    else:
        o += ['<path d="M50 106 L64 130 L78 106 Z" fill="#f2f2f2"/>',
              f'<path d="M61 112 L67 112 L68 140 L64 146 L60 140 Z" fill="{grav}"/>',
              f'<path d="M44 106 L58 150 L40 160 L18 160 L24 124 Z" fill="{cor_corpo}" stroke="#00000066"/><path d="M84 106 L70 150 L88 160 L110 160 L104 124 Z" fill="{cor_corpo}" stroke="#00000066"/>']
    # cabeça
    o.append(f'<ellipse cx="64" cy="64" rx="25" ry="31" fill="{pele}"/>')
    o.append(f'<ellipse cx="39" cy="66" rx="4" ry="7" fill="{pele}"/><ellipse cx="89" cy="66" rx="4" ry="7" fill="{pele}"/>')
    # cabelo
    if estilo == 0:
        o.append(f'<path d="M39 58 Q40 30 64 30 Q88 30 89 58 Q80 44 64 44 Q48 44 39 58 Z" fill="{cab}"/>')
    elif estilo == 1:
        o.append(f'<path d="M38 60 Q36 28 66 29 Q90 30 90 56 Q76 40 56 46 Q44 50 38 60 Z" fill="{cab}"/>')
    elif estilo == 2:
        o.append(f'<path d="M39 56 Q38 46 44 44 Q42 52 41 60 Z" fill="{cab}"/><path d="M89 56 Q90 46 84 44 Q86 52 87 60 Z" fill="{cab}"/>')
    else:
        o.append(f'<path d="M36 62 Q30 24 64 25 Q98 24 92 62 Q88 42 64 42 Q40 42 36 62 Z" fill="{cab}"/>')
    # rosto
    o += ['<circle cx="54" cy="64" r="2.4" fill="#1a1a1a"/><circle cx="74" cy="64" r="2.4" fill="#1a1a1a"/>',
          f'<path d="M48 57 L60 56" stroke="{cab}" stroke-width="2.4"/><path d="M68 56 L80 57" stroke="{cab}" stroke-width="2.4"/>',
          '<path d="M64 66 L61 76 L66 77" stroke="#00000044" stroke-width="1.6" fill="none"/>',
          '<path d="M56 85 Q64 89 72 85" stroke="#7a3b30" stroke-width="2.2" fill="none"/>']
    if h(i, 7) % 5 == 0:
        o.append(f'<path d="M56 82 Q64 78 72 82 Q64 80 56 82 Z" fill="{cab}" stroke="{cab}" stroke-width="2"/>')
    if h(i, 8) % 6 == 0:
        o.append('<g fill="none" stroke="#222" stroke-width="1.6"><circle cx="54" cy="64" r="7"/><circle cx="74" cy="64" r="7"/><path d="M61 64 L67 64"/></g>')
    if mil and h(i, 10) % 2 == 0:
        o.append(f'<path d="M38 44 Q64 22 90 44 L90 50 Q64 38 38 50 Z" fill="{cor_corpo}" stroke="#00000066"/><rect x="58" y="34" width="12" height="7" fill="#c9a13b"/>')
    # distintivo de iniciais
    o += ['<rect x="0" y="136" width="128" height="24" fill="#000000aa"/>',
          f'<text x="64" y="154" font-family="DejaVu Sans, Arial, sans-serif" font-size="16" font-weight="bold" fill="#ffffff" text-anchor="middle">{iniciais(p["nome"])}</text>',
          '<rect x="0.5" y="0.5" width="127" height="159" fill="none" stroke="#e8dcc0" stroke-width="2"/>', '</svg>']
    return "\n".join(o)

def carregar():
    sys.path.insert(0, "/home/rafael/claude/guerra-fria-staging/personagens")
    from validar import SimpleRonParser
    ps = []
    for f in ("personagens.ron", "stubs_lideres.ron"):
        ps += SimpleRonParser(open(os.path.join(DADOS, f), encoding="utf-8").read()).parse()
    return ps

def main():
    os.makedirs(SAIDA, exist_ok=True)
    ps = carregar() + [{"id": "retrato_generico", "nome": "?", "ideologia": "x", "cargo_inicial": "ministro", "nascimento": "1900"}]
    for p in ps:
        s = os.path.join(SAIDA, p["id"] + ".svg")
        open(s, "w", encoding="utf-8").write(svg(p))
        subprocess.run(["rsvg-convert", "-w", "128", "-h", "160", "-o", s[:-4] + ".png", s], check=True)
        os.remove(s)  # só o PNG vai para o repositório
    print(len(ps), "retratos em", SAIDA)

if __name__ == "__main__":
    main()
