import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

def parse_provs(text):
    # Parse provinces
    # We want to extract for each province:
    # id, pais_id, terra_aravel_total, limites_recursos, pops, predios
    provs = []
    # Find all (id:"...", ... predios:[...])
    # Regex for province
    pattern = re.compile(r'\(id:"(?P<id>[^"]+)",nome:"(?P<nome>[^"]*)",.*?pais_id:"(?P<pais_id>[^"]+)",(?P<rest>.*?)\),\s*(?=\(id:|$|\])', re.DOTALL)
    for m in pattern.finditer(text):
        p_id = m.group("id")
        p_nome = m.group("nome")
        p_pais = m.group("pais_id")
        rest = m.group("rest")
        
        # Extract predios
        pred_match = re.search(r'predios:\[(.*?)\]', rest, re.DOTALL)
        predios = []
        if pred_match:
            p_content = pred_match.group(1)
            # Find (tipo_predio_id:"...",nivel:\d+,...)
            for b in re.finditer(r'\(tipo_predio_id:"(?P<tipo>[^"]+)",nivel:(?P<nivel>\d+)', p_content):
                predios.append((b.group("tipo"), int(b.group("nivel"))))
        
        provs.append({
            "id": p_id,
            "nome": p_nome,
            "pais_id": p_pais,
            "predios": predios
        })
    return provs

print("Reading jogavel...")
proc_j = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
provs_j = parse_provs(proc_j.stdout)
print(f"jogavel: parsed {len(provs_j)} provinces")

total_predios_j = sum(len(p["predios"]) for p in provs_j)
total_niveis_j = sum(sum(b[1] for b in p["predios"]) for p in provs_j)
print(f"jogavel: total building instances={total_predios_j}, total levels={total_niveis_j}")

by_country_j = defaultdict(lambda: {"count": 0, "niveis": 0})
for p in provs_j:
    c = p["pais_id"]
    by_country_j[c]["count"] += len(p["predios"])
    by_country_j[c]["niveis"] += sum(b[1] for b in p["predios"])

for c in ["usa", "sov", "gbr", "bra", "ger", "jap", "fra", "chi", "ind"]:
    print(f"  {c}: instances={by_country_j[c]['count']}, niveis={by_country_j[c]['niveis']}")

print("\nReading 1bdd42ac...")
proc_1b = subprocess.run(["git", "show", "1bdd42ac:assets/data/economia/provincias.ron"], capture_output=True, text=True)
provs_1b = parse_provs(proc_1b.stdout)
print(f"1bdd42ac: parsed {len(provs_1b)} provinces")
total_predios_1b = sum(len(p["predios"]) for p in provs_1b)
total_niveis_1b = sum(sum(b[1] for b in p["predios"]) for p in provs_1b)
print(f"1bdd42ac: total building instances={total_predios_1b}, total levels={total_niveis_1b}")
by_country_1b = defaultdict(lambda: {"count": 0, "niveis": 0})
for p in provs_1b:
    c = p["pais_id"]
    by_country_1b[c]["count"] += len(p["predios"])
    by_country_1b[c]["niveis"] += sum(b[1] for b in p["predios"])

for c in ["usa", "sov", "gbr", "bra", "ger", "jap", "fra", "chi", "ind"]:
    print(f"  {c}: instances={by_country_1b[c]['count']}, niveis={by_country_1b[c]['niveis']}")
