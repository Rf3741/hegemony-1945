import re
from collections import defaultdict

# 1. Estados em assets/data/economia/estados.ron
eco_estados = {}
with open("/home/rafael/claude/guerra-fria-jogar/assets/data/economia/estados.ron", "r") as f:
    for m in re.finditer(r'\(\s*id:\s*"([^"]+)",\s*nome:\s*"([^"]+)",\s*pais_id:\s*"([^"]+)"', f.read()):
        eco_estados[m.group(1)] = {"nome": m.group(2), "pais_id": m.group(3)}

# 2. Provincias em assets/data/economia/provincias.ron
# Each prov has id, estado_id, terra_aravel_total, limites_recursos, predios
prov_por_estado = defaultdict(list)
with open("/home/rafael/claude/guerra-fria-jogar/assets/data/economia/provincias.ron", "r") as f:
    content = f.read()

# Let's see how provincias are formatted
prov_matches = list(re.finditer(r'id:"([^"]+)",nome:"([^"]+)",(?:codigo_externo:Some\("([^"]+)"\),)?estado_id:"([^"]+)"', content))
print(f"Total provs lidas: {len(prov_matches)}")

# Check match between eco_estados and staging territorio estados
staging_estados_path = "/home/rafael/claude/guerra-fria-staging/territorio_dados/estados.ron"
staging_estados = {}
with open(staging_estados_path, "r") as f:
    st_content = f.read()
    for m in re.finditer(r'"([^"]+)":\s*\(\s*id:\s*"([^"]+)",\s*nome:\s*"([^"]+)"', st_content):
        staging_estados[m.group(1)] = m.group(3)

print(f"Total eco_estados: {len(eco_estados)}")
print(f"Total staging_estados: {len(staging_estados)}")

overlap = set(eco_estados.keys()) & set(staging_estados.keys())
print(f"Overlap de IDs de estados: {len(overlap)} de {len(eco_estados)}")

diff = set(eco_estados.keys()) - set(staging_estados.keys())
print(f"Estados em eco mas não em staging: {len(diff)}")
if diff:
    print(f"Exemplos de diff: {list(diff)[:10]}")
