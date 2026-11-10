import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

pop_by_type = defaultdict(float)
emp_by_type = defaultdict(float)

lines = j_text.strip().split("\n")
for line in lines:
    if 'pais_id:"usa"' not in line:
        continue
    m_pops = re.search(r'pops:\[(.*?)\]', line)
    if m_pops:
        for pop_m in re.finditer(r'tipo_pop_id:"([^"]+)",.*?populacao:([0-9.]+),empregados:([0-9.]+)', m_pops.group(1)):
            tp = pop_m.group(1)
            pop = float(pop_m.group(2))
            emp = float(pop_m.group(3))
            pop_by_type[tp] += pop
            emp_by_type[tp] += emp

for tp in sorted(pop_by_type.keys()):
    p = pop_by_type[tp]
    e = emp_by_type[tp]
    rate = e / p if p > 0 else 0
    print(f"USA {tp:15s}: pop={p/1e6:6.2f}M, emp={e/1e6:6.2f}M, rate={rate*100:5.1f}%, desemp={100-rate*100:5.1f}%")
