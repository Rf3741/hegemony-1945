import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")
print("Running inspect...")
# Get total buildings count from current file
with open("assets/data/economia/provincias.ron", "r", encoding="utf-8") as f:
    content = f.read()

predio_count = content.count("tipo_predio_id:")
prov_count = content.count('(id:"')
print(f"Current provincias.ron: {prov_count} provinces, {predio_count} buildings")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

# Count buildings and levels per country in jogavel
total_niveis = 0
total_predios = 0
niveis_por_pais = defaultdict(int)
niveis_por_pais_tipo = defaultdict(lambda: defaultdict(int))
predios_por_pais = defaultdict(int)

# In jogavel, each province is one line!
lines = j_text.strip().split("\n")
print(f"Total lines in jogavel: {len(lines)}")
for line in lines:
    line = line.strip()
    if not line.startswith("(") or not line.endswith("),"):
        continue
    # find pais_id
    m_pais = re.search(r'pais_id:"([^"]+)"', line)
    if not m_pais:
        continue
    pais = m_pais.group(1)
    
    # find all predios
    m_predios = re.search(r'predios:\[(.*)\]', line)
    if m_predios:
        p_str = m_predios.group(1)
        for b in re.finditer(r'\(tipo_predio_id:"([^"]+)",nivel:(\d+)', p_str):
            tipo = b.group(1)
            nivel = int(b.group(2))
            total_predios += 1
            total_niveis += nivel
            predios_por_pais[pais] += 1
            niveis_por_pais[pais] += nivel
            niveis_por_pais_tipo[pais][tipo] += nivel

print(f"Total buildings: {total_predios}, Total levels: {total_niveis}")
print("\nUSA building levels in jogavel:")
for tipo, lvl in sorted(niveis_por_pais_tipo["usa"].items(), key=lambda x: -x[1]):
    print(f"  {tipo}: {lvl}")






