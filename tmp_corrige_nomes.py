import pathlib

RAIZ = pathlib.Path("/home/rafael/claude/hege-wt/q-nomes")
ARQ = [
    "crates/bandeiras_resolver/variantes.ron",
    "assets/data/politica/paises_politica.ron",
    "assets/data/economia/paises.ron",
    "assets/data/economia/mercados.ron",
    "assets/data/tecnologia/tecnologias_iniciais.ron",
    "data/scenario_1945/countries_1945.ron",
]

# (antigo, novo) — nomes oficiais, sem o sufixo de ocupação
REGRAS = [
    ("Reino da Dinamarca (Sob Ocupação Alemã)", "Reino da Dinamarca"),
    ("Império do Irã (Sob Ocupação Anglo-Soviética)", "Império do Irã"),
    ("Hong Kong (Sob Ocupação Japonesa)", "Hong Kong"),
    ("Índias Orientais Holandesas (Sob Ocupação Japonesa)", "Índias Orientais Holandesas"),
    ("Malásia Britânica (Sob Ocupação Japonesa)", "Malásia Britânica"),
    ("Brunei Britânico (Sob Ocupação Japonesa)", "Brunei Britânico"),
    ("Timor Português (Sob Ocupação Japonesa)", "Timor Português"),
    ("República da China (Nacionalista)", "República da China"),
    ("Estados Unidos do Brasil (Estado Novo)", "Estados Unidos do Brasil"),
    ('nome: "Dinamarca"', 'nome: "Reino da Dinamarca"'),
    ('nome: "Irã (Ocupação Anglo-Soviética)"', 'nome: "Império do Irã"'),
    ("Junta Militar da China", "República da China"),
    ("Junta Militar do Brasil", "Estados Unidos do Brasil"),
]

for a in ARQ:
    p = RAIZ / a
    t = p.read_text(encoding="utf-8")
    original = t
    for antigo, novo in REGRAS:
        if antigo in t:
            n = t.count(antigo)
            t = t.replace(antigo, novo)
            print(f"{a}: {antigo!r} -> {novo!r} ({n}x)")
    if t != original:
        p.write_text(t, encoding="utf-8")
