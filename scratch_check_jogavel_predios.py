import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

predios_per_prov = []
zero_level_predios = 0
min_level = 999999
types_in_j = set()

for line in j_text.split("\n"):
    line = line.strip()
    if not line.startswith("(") or not line.endswith("),"):
        continue
    m_pred = re.search(r'predios:\[(.*)\]', line)
    if m_pred:
        b_count = 0
        for b in re.finditer(r'tipo_predio_id:"([^"]+)",nivel:(\d+)', m_pred.group(1)):
            b_count += 1
            types_in_j.add(b.group(1))
            lvl = int(b.group(2))
            if lvl == 0:
                zero_level_predios += 1
            else:
                min_level = min(min_level, lvl)
        predios_per_prov.append(b_count)

print(f"Total provinces: {len(predios_per_prov)}")
print(f"Average buildings per prov: {sum(predios_per_prov)/len(predios_per_prov):.1f}")
print(f"Min buildings in a prov: {min(predios_per_prov)}, Max: {max(predios_per_prov)}")
print(f"Zero level buildings in jogavel: {zero_level_predios}")
print(f"Min level in jogavel: {min_level}")
print(f"Total unique building types: {len(types_in_j)}")
