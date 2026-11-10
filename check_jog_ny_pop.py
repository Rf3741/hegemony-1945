import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

for line in proc.stdout.splitlines():
    if 'Nova Iorque' in line and 'pais_id:"usa"' in line:
        pop = sum(float(m.group(1)) for m in re.finditer(r'populacao:([0-9.]+)', line))
        emp = sum(float(m.group(1)) for m in re.finditer(r'empregados:([0-9.]+)', line))
        print(f"JOGAVEL NY: pop={pop:.0f}, emp={emp:.0f}")
