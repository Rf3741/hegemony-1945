import re
from collections import defaultdict

# 1. Carregar regioes_1945_canonico.ron
canonico = {}
canonico_recursos = {}
canonico_path = "/home/rafael/claude/guerra-fria-staging/provincias_1945/regioes_1945_canonico.ron"

with open(canonico_path, "r", encoding="utf-8") as f:
    text = f.read()

# Pattern for entry: "ADM1-CODE": ( ... recursos: { ... } ... )
entries = re.findall(r'"([A-Z]{3}-\d+)":\s*\((.*?)\),', text, re.DOTALL)
print(f"Total regioes canonicas lidas: {len(entries)}")

for adm1, body in entries:
    recs = {}
    m_rec = re.search(r'recursos:\s*\{([^}]*)\}', body)
    if m_rec:
        for it in m_rec.group(1).split(","):
            parts = it.split(":")
            if len(parts) == 2:
                k = parts[0].strip().strip('"')
                v = int(parts[1].strip())
                recs[k] = v
    canonico_recursos[adm1] = recs

# 2. Carregar staging estados.ron
staging_path = "/home/rafael/claude/guerra-fria-staging/territorio_dados/estados.ron"
with open(staging_path, "r", encoding="utf-8") as f:
    st_text = f.read()

st_estados = re.findall(r'"([^"]+)":\s*\((.*?)\),', st_text, re.DOTALL)
print(f"Total estados staging lidos: {len(st_estados)}")

for sid, body in st_estados:
    if sid in ("bra_minas_gerais", "bra_sao_paulo", "bra_goias", "usa_texas", "ger_nordrhein_westfalen", "donbass"):
        nome_m = re.search(r'nome:\s*"([^"]+)"', body)
        clima_m = re.search(r'clima:\s*"([^"]+)"', body)
        terreno_m = re.search(r'terreno_predominante:\s*"([^"]+)"', body)
        admin1s = re.findall(r'"([A-Z]{3}-\d+)"', body)
        
        recs_agregados = defaultdict(int)
        for adm in admin1s:
            for rk, rv in canonico_recursos.get(adm, {}).items():
                recs_agregados[rk] += rv
        
        print(f"\nEstado {sid} ({nome_m.group(1) if nome_m else ''}):")
        print(f"  Clima: {clima_m.group(1) if clima_m else ''}, Terreno: {terreno_m.group(1) if terreno_m else ''}")
        print(f"  Admin1 count: {len(admin1s)}")
        print(f"  Recursos agregados canonico: {dict(recs_agregados)}")
