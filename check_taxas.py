import subprocess
import re
from collections import defaultdict

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

taxas = defaultdict(list)
for line in proc.stdout.splitlines():
    if 'pais_id:"usa"' in line:
        for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
            tp, pop, emp = m.group(1), float(m.group(2)), float(m.group(3))
            if pop > 0:
                taxas[tp].append(emp / pop)

for tp, vals in sorted(taxas.items()):
    print(f"{tp:20}: min={min(vals):.4f}, max={max(vals):.4f}, avg={sum(vals)/len(vals):.4f}")
