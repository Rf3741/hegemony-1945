import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

# Load potenciais.ron
with open("assets/data/economia/potenciais.ron", "r", encoding="utf-8") as f:
    pot_text = f.read()

# Parse potenciais
# estado_id -> { terra_aravel, culturas: [], recursos: {rec: lim}, qualidade: {rec: q} }
potenciais = {}
for m in re.finditer(r'estado_id:\s*"([^"]+)",\s*terra_aravel:\s*(\d+),\s*culturas_permitidas:\s*\[(.*?)\](?:,\s*recursos:\s*\{(.*?)\})?(?:,\s*qualidade_recursos:\s*\{(.*?)\})?', pot_text, re.DOTALL):
    est_id = m.group(1)
    terra = int(m.group(2))
    culturas = [c.strip(' "\'') for c in m.group(3).split(',') if c.strip(' "\'')]
    recursos = {}
    if m.group(4):
        for rm in re.finditer(r'"([^"]+)":\s*(\d+)', m.group(4)):
            recursos[rm.group(1)] = int(rm.group(2))
    qualidade = {}
    if m.group(5):
        for qm in re.finditer(r'"([^"]+)":\s*([0-9.]+)', m.group(5)):
            qualidade[qm.group(1)] = float(qm.group(2))
    potenciais[est_id] = {
        "terra_aravel": terra,
        "culturas": culturas,
        "recursos": recursos,
        "qualidade": qualidade,
    }

print(f"Loaded {len(potenciais)} potenciais")

def recurso_do_predio(tipo):
    if tipo == "mina_ferro": return "ferro"
    if tipo == "mina_carvao": return "carvao"
    if tipo == "poco_petroleo": return "petroleo"
    if tipo == "campo_gas_natural": return "gas_natural"
    if tipo == "mina_cobre": return "cobre"
    if tipo == "mina_uranio": return "uranio"
    if tipo == "mina_bauxita": return "bauxita"
    if tipo == "porto_pesca": return "pesca"
    if tipo == "serraria": return "madeira"
    return None

# Check how many buildings in jogavel have recurso == 0
total_b = 0
invalid_extrativos = 0

for line in j_text.split("\n"):
    line = line.strip()
    if not line.startswith("(") or not line.endswith("),"):
        continue
    m_est = re.search(r'estado_id:"([^"]+)"', line)
    m_lim = re.search(r'limites_recursos:\{(.*?)\}', line)
    prov_recursos = {}
    if m_lim and m_lim.group(1):
        for rm in re.finditer(r'"([^"]+)":(\d+)', m_lim.group(1)):
            prov_recursos[rm.group(1)] = int(rm.group(2))
    est_id = m_est.group(1) if m_est else ""
    pot = potenciais.get(est_id, {})
    pot_recursos = pot.get("recursos", {})
    
    m_pred = re.search(r'predios:\[(.*)\]', line)
    if m_pred:
        for b in re.finditer(r'tipo_predio_id:"([^"]+)"', m_pred.group(1)):
            tp = b.group(1)
            total_b += 1
            rec = recurso_do_predio(tp)
            if rec:
                lim_prov = prov_recursos.get(rec, 0)
                lim_est = pot_recursos.get(rec, 0)
                if lim_prov == 0 or lim_est == 0:
                    invalid_extrativos += 1

print(f"Total buildings: {total_b}")
print(f"Extrativos with 0 resource in province/state: {invalid_extrativos}")
print(f"Remaining buildings if invalid extrativos removed: {total_b - invalid_extrativos}")
