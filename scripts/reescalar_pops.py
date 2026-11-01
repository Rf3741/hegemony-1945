#!/usr/bin/env python3
"""Reescala pops e predios de provincias.ron para a populacao historica de 1945, por pais."""
import re, sys
from collections import defaultdict

ALVO14 = {"usa":140,"sov":170,"chi":500,"ind":400,"bra":46,"gbr":48,"ger":65,"jap":72,"fra":40,"ita":45,"can":12,"arg":15,"pol":24,"tur":19}
CAM = "assets/data/economia/provincias.ron"
alvos = {}
txt = open("/home/rafael/claude/guerra-fria-staging/nacoes/paises.ron", encoding="utf-8").read()
for b in re.findall(r'Pais\s*\((.*?)\n\s*\)', txt, re.DOTALL):
    t = re.search(r'tag:\s*"([^"]+)"', b); p = re.search(r'populacao_total:\s*(\d+)', b)
    if t and p: alvos[t.group(1).lower()] = float(p.group(1))
for k, v in ALVO14.items(): alvos[k] = v * 1e6

linhas = open(CAM, encoding="utf-8").read().split("\n")
POP = re.compile(r'(populacao:)([0-9.]+)(,empregados:)([0-9.]+)(,riqueza_reservas:)([0-9.]+)')
atual = defaultdict(float)
for l in linhas:
    if l.startswith("    (id:"):
        pa = re.search(r'pais_id:"([^"]+)"', l).group(1)
        for m in POP.finditer(l): atual[pa] += float(m.group(2))
K = {}
for pa, cur in atual.items():
    a = alvos.get(pa)
    K[pa] = a / cur if a and a >= 5e5 and cur > 0 and abs(a / cur - 1) > 0.05 else 1.0

PRED = re.compile(r'(nivel:)(\d+)(,metodos_ativos:\[.*?\],empregados_por_tipo:\{)(.*?)(\},salario_base:[0-9.]+,reservas_caixa:)([0-9.]+)')
def f(x): return f"{x:.1f}"
def novo(l):
    pa = re.search(r'pais_id:"([^"]+)"', l).group(1); k = K[pa]
    if k == 1.0: return l
    i = l.find("predios:[")
    pops, pred = l[:i], l[i:]
    pops = POP.sub(lambda m: m.group(1)+f(float(m.group(2))*k)+m.group(3)+f(float(m.group(4))*k)+m.group(5)+f(float(m.group(6))*k), pops)
    def rp(m):
        n = int(m.group(2)); nn = max(1, round(n*k)); r = nn/n
        emp = re.sub(r'(":)([0-9.]+)', lambda e: e.group(1)+f(float(e.group(2))*r), m.group(4))
        return m.group(1)+str(nn)+m.group(3)+emp+m.group(5)+f(float(m.group(6))*r)
    return pops + PRED.sub(rp, pred)
saida = [novo(l) if l.startswith("    (id:") else l for l in linhas]
open(CAM, "w", encoding="utf-8").write("\n".join(saida))
for p in ["usa","sov","chi","ind","bra","gbr"]: print(p, K[p])

# Valores absolutos por pais em paises.ron (tesouro, divida, credito, fundo privado, PIB)
CP = "assets/data/economia/paises.ron"
atual_id = None
out = []
for l in open(CP, encoding="utf-8").read().split("\n"):
    m = re.match(r'^        id: "([^"]+)",', l)
    if m: atual_id = m.group(1)
    m = re.match(r'^(\s+(?:tesouro|divida|limite_credito|fundo_investimento_privado|pib_diario): )([0-9.]+)(,)$', l)
    if m and atual_id in K and K[atual_id] != 1.0:
        l = m.group(1) + f(float(m.group(2)) * K[atual_id]) + m.group(3)
    out.append(l)
open(CP, "w", encoding="utf-8").write("\n".join(out))
