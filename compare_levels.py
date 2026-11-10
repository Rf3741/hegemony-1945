import subprocess
import re
from collections import defaultdict

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

jog_levels = defaultdict(int)
for line in proc.stdout.splitlines():
    if 'pais_id:"usa"' in line:
        for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)', line):
            jog_levels[m.group(1)] += int(m.group(2))

cur_levels = defaultdict(int)
with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'pais_id:"usa"' in line:
            for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)', line):
                cur_levels[m.group(1)] += int(m.group(2))

print(f"{'Tipo':30} {'Jogavel':>10} {'Current':>10} {'Diff':>10}")
for tipo in sorted(set(jog_levels.keys()) | set(cur_levels.keys())):
    j = jog_levels[tipo]
    c = cur_levels[tipo]
    if j != c:
        print(f"{tipo:30} {j:10d} {c:10d} {c - j:10d}")
