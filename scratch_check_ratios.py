import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

ratios_by_type = defaultdict(list)
ratios_by_prov_pop = {}

lines = j_text.strip().split("\n")
for line in lines:
    line = line.strip()
    if not line.startswith("(") or not line.endswith("),"):
        continue
    m_id = re.search(r'id:"([^"]+)"', line)
    m_pais = re.search(r'pais_id:"([^"]+)"', line)
    if not m_id or not m_pais:
        continue
    pid = m_id.group(1)
    pais = m_pais.group(1)
    if pais != "usa":
        continue
    
    m_pops = re.search(r'pops:\[(.*?)\]', line)
    if m_pops:
        for pop_m in re.finditer(r'tipo_pop_id:"([^"]+)",.*?populacao:([0-9.]+),empregados:([0-9.]+)', m_pops.group(1)):
            tp = pop_m.group(1)
            pop = float(pop_m.group(2))
            emp = float(pop_m.group(3))
            ratio = emp / pop if pop > 0 else 0.0
            ratios_by_type[tp].append(ratio)
            ratios_by_prov_pop[(pid, tp)] = ratio

for tp, r_list in sorted(ratios_by_type.items()):
    avg_r = sum(r_list) / len(r_list)
    min_r = min(r_list)
    max_r = max(r_list)
    print(f"USA {tp}: count={len(r_list)}, avg_ratio={avg_r:.4f}, min={min_r:.4f}, max={max_r:.4f}")
